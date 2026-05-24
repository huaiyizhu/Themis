use std::sync::Arc;
use themis_audio::{AudioSource, SystemAudioOptions};
use themis_analysis::create_analyzer;
use themis_azure::create_recognizer;
use chrono::Utc;
use themis_core::{
    normalize_pcm16, AnalysisDiagnostics, AudioFrame, CaptureDiagnostics, CaptureState,
    LatencyDiagnostics, StateMachine, ThemisConfig, TranscriptEvent,
};
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::{error, info};

pub struct CaptureEngine {
    config: ThemisConfig,
    state: Arc<StateMachine>,
    transcript_tx: broadcast::Sender<TranscriptEvent>,
    capture_diag: Arc<CaptureDiagnostics>,
    latency_diag: Arc<LatencyDiagnostics>,
    analysis_diag: Arc<AnalysisDiagnostics>,
    inner: Mutex<Option<RunningCapture>>,
}

struct RunningCapture {
    stop_tx: mpsc::Sender<()>,
    _audio: Box<dyn AudioSource + Send>,
    _tasks: RunningTasks,
}

struct RunningTasks {
    speech: tokio::task::JoinHandle<()>,
    events: tokio::task::JoinHandle<()>,
}

impl RunningTasks {
    async fn join_all(self) {
        let RunningTasks { speech, events } = self;
        let _ = speech.await;
        events.abort();
        let _ = events.await;
    }
}

impl CaptureEngine {
    pub fn new(
        config: ThemisConfig,
        state: Arc<StateMachine>,
        capture_diag: Arc<CaptureDiagnostics>,
        latency_diag: Arc<LatencyDiagnostics>,
        analysis_diag: Arc<AnalysisDiagnostics>,
    ) -> Self {
        let (transcript_tx, _) = broadcast::channel(256);
        Self {
            config,
            state,
            transcript_tx,
            capture_diag,
            latency_diag,
            analysis_diag,
            inner: Mutex::new(None),
        }
    }

    pub fn latency_diagnostics(&self) -> Arc<LatencyDiagnostics> {
        Arc::clone(&self.latency_diag)
    }

    pub fn transcript_sender(&self) -> broadcast::Sender<TranscriptEvent> {
        self.transcript_tx.clone()
    }

    pub fn capture_diagnostics(&self) -> Arc<CaptureDiagnostics> {
        Arc::clone(&self.capture_diag)
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;
        if guard.is_some() {
            anyhow::bail!("capture already running");
        }

        self.capture_diag.reset_session_peak();
        self.analysis_diag.clear();

        let (frame_tx, frame_rx) = mpsc::channel::<AudioFrame>(256);
        let (stop_tx, stop_rx) = mpsc::channel::<()>(1);

        let mut audio = themis_audio::create_loopback(
            self.config.sample_rate,
            self.config.channels,
            SystemAudioOptions {
                output_device: self.config.audio_output_device.clone(),
                capture_mode: self.config.audio_capture_mode.clone(),
                gain_max: self.config.audio_gain_max,
                diagnostics: Some(Arc::clone(&self.capture_diag)),
            },
        )?;
        audio.start(frame_tx)?;

        const STT_SAMPLE_RATE: u32 = 16_000;
        let gain_max = self.config.audio_gain_max;
        let mut speech = create_recognizer(&self.config);
        let mut speech_events = speech.subscribe();
        speech.start().await?;

        let state = Arc::clone(&self.state);
        let transcript_tx = self.transcript_tx.clone();
        let analysis = create_analyzer(&self.config);

        let speech_handle = tokio::spawn(async move {
            let mut frame_rx = frame_rx;
            let mut stop_rx = stop_rx;
            loop {
                tokio::select! {
                    _ = stop_rx.recv() => break,
                    frame = frame_rx.recv() => {
                        match frame {
                            Some(f) => {
                                let mut pcm = f.to_mono_pcm16(STT_SAMPLE_RATE);
                                normalize_pcm16(&mut pcm, 12_000, gain_max);
                                if let Err(e) = speech.push_audio(&pcm).await {
                                    error!(error = %e, "push_audio failed");
                                }
                            }
                            None => break,
                        }
                    }
                }
            }
            let _ = speech.stop().await;
        });

        let latency_diag = Arc::clone(&self.latency_diag);
        let analysis_diag = Arc::clone(&self.analysis_diag);
        let event_handle = tokio::spawn(async move {
            while let Ok(ev) = speech_events.recv().await {
                if ev.is_final {
                    state.record_transcript();
                }
                let latency = ev.latency.clone();
                let emitted_unix_ms = Utc::now().timestamp_millis();
                if ev.is_final {
                    if let Some(ref breakdown) = latency {
                        latency_diag.push(ev.text.clone(), true, breakdown.clone());
                    }
                }

                if ev.is_final {
                    let text = ev.text.clone();
                    let tx = transcript_tx.clone();
                    let analysis = Arc::clone(&analysis);
                    // Show transcript immediately; attach insights when ready.
                    let _ = tx.send(TranscriptEvent {
                        text: text.clone(),
                        is_final: true,
                        feedback: None,
                        insights: None,
                        emitted_unix_ms,
                        latency: latency.clone(),
                    });
                    let analysis_diag = Arc::clone(&analysis_diag);
                    tokio::spawn(async move {
                        let detail = analysis.analyze(&text).await.ok().flatten();
                        if let Some(ref d) = detail {
                            analysis_diag.push(text.clone(), d);
                        }
                        let merged = detail.as_ref().map(|d| d.merged.clone());
                        let feedback = merged.as_ref().map(|i| i.summary());
                        let _ = tx.send(TranscriptEvent {
                            text,
                            is_final: true,
                            feedback,
                            insights: merged,
                            emitted_unix_ms: Utc::now().timestamp_millis(),
                            latency,
                        });
                    });
                } else {
                    let _ = transcript_tx.send(TranscriptEvent {
                        text: ev.text,
                        is_final: false,
                        feedback: None,
                        insights: None,
                        emitted_unix_ms,
                        latency,
                    });
                }
            }
        });

        *guard = Some(RunningCapture {
            stop_tx,
            _audio: audio,
            _tasks: RunningTasks {
                speech: speech_handle,
                events: event_handle,
            },
        });

        self.state
            .set_state(CaptureState::Capturing, "capture engine running");
        info!("capture engine started");
        Ok(())
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;
        if let Some(mut running) = guard.take() {
            let _ = running.stop_tx.send(()).await;
            running._tasks.join_all().await;
            running._audio.stop()?;
        }
        self.state.set_state(CaptureState::Idle, "capture stopped");
        info!("capture engine stopped");
        Ok(())
    }
}

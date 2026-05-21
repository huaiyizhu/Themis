use std::sync::Arc;
use themis_audio::AudioSource;
use themis_azure::create_recognizer;
use themis_core::{
    AnalysisProvider, AudioFrame, CaptureState, NoopAnalysis, StateMachine, ThemisConfig,
    TranscriptEvent,
};
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::{error, info};

pub struct CaptureEngine {
    config: ThemisConfig,
    state: Arc<StateMachine>,
    transcript_tx: broadcast::Sender<TranscriptEvent>,
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
    pub fn new(config: ThemisConfig, state: Arc<StateMachine>) -> Self {
        let (transcript_tx, _) = broadcast::channel(256);
        Self {
            config,
            state,
            transcript_tx,
            inner: Mutex::new(None),
        }
    }

    pub fn transcript_sender(&self) -> broadcast::Sender<TranscriptEvent> {
        self.transcript_tx.clone()
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;
        if guard.is_some() {
            anyhow::bail!("capture already running");
        }

        let (frame_tx, frame_rx) = mpsc::channel::<AudioFrame>(256);
        let (stop_tx, stop_rx) = mpsc::channel::<()>(1);

        let mut audio =
            themis_audio::create_loopback(self.config.sample_rate, self.config.channels)?;
        audio.start(frame_tx)?;

        let mut speech = create_recognizer(&self.config);
        speech.start().await?;

        let sample_rate = self.config.sample_rate;
        let state = Arc::clone(&self.state);
        let transcript_tx = self.transcript_tx.clone();
        let analysis: Arc<dyn AnalysisProvider> = Arc::new(NoopAnalysis);
        let mut speech_events = speech.subscribe();

        let speech_handle = tokio::spawn(async move {
            let mut frame_rx = frame_rx;
            let mut stop_rx = stop_rx;
            loop {
                tokio::select! {
                    _ = stop_rx.recv() => break,
                    frame = frame_rx.recv() => {
                        match frame {
                            Some(f) => {
                                let pcm = f.to_mono_pcm16(sample_rate);
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

        let event_handle = tokio::spawn(async move {
            while let Ok(ev) = speech_events.recv().await {
                state.record_transcript();
                let feedback = if ev.is_final {
                    analysis.analyze(&ev.text).await.ok().flatten()
                } else {
                    None
                };
                let _ = transcript_tx.send(TranscriptEvent {
                    text: ev.text,
                    is_final: ev.is_final,
                    feedback,
                });
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

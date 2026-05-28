#import "themis_tap.h"

#import <AudioToolbox/AudioToolbox.h>
#import <CoreAudio/AudioHardwareTapping.h>
#import <CoreAudio/CATapDescription.h>
#import <CoreAudio/CoreAudio.h>
#import <Foundation/Foundation.h>
#import <stdatomic.h>
#import <stdlib.h>
#import <string.h>

typedef struct ThemisTapState {
  AudioObjectID tap_id;
  AudioObjectID aggregate_id;
  AudioDeviceIOProcID io_proc;
  atomic_bool running;
  ThemisTapCallback callback;
  void *userdata;
  char detail[512];
  AudioStreamBasicDescription device_format;
} ThemisTapState;

static AudioObjectPropertyAddress Prop(AudioObjectPropertySelector sel) {
  AudioObjectPropertyAddress addr = {0};
  addr.mSelector = sel;
  addr.mScope = kAudioObjectPropertyScopeGlobal;
  addr.mElement = kAudioObjectPropertyElementMain;
  return addr;
}

static OSStatus GetTapUid(AudioObjectID tap_id, CFStringRef *out_uid) {
  AudioObjectPropertyAddress addr = Prop(kAudioTapPropertyUID);
  UInt32 size = sizeof(CFStringRef);
  return AudioObjectGetPropertyData(tap_id, &addr, 0, NULL, &size, out_uid);
}

static void SetDetail(ThemisTapState *tap, const char *msg) {
  if (!tap || !msg) {
    return;
  }
  strncpy(tap->detail, msg, sizeof(tap->detail) - 1);
  tap->detail[sizeof(tap->detail) - 1] = '\0';
}

static OSStatus IOProc(AudioObjectID inDevice, const AudioTimeStamp *inNow,
                       const AudioBufferList *inInputData,
                       const AudioTimeStamp *inInputTime,
                       AudioBufferList *outOutputData,
                       const AudioTimeStamp *inOutputTime, void *inClientData) {
  (void)inDevice;
  (void)inNow;
  (void)inInputTime;
  (void)outOutputData;
  (void)inOutputTime;

  ThemisTapState *tap = (ThemisTapState *)inClientData;
  if (!tap || !atomic_load(&tap->running) || !tap->callback || !inInputData) {
    return noErr;
  }
  if (inInputData->mNumberBuffers == 0) {
    return noErr;
  }

  const AudioBuffer *buf = &inInputData->mBuffers[0];
  if (!buf->mData || buf->mDataByteSize == 0) {
    return noErr;
  }

  const uint32_t rate = (uint32_t)tap->device_format.mSampleRate;
  const uint16_t channels = (uint16_t)tap->device_format.mChannelsPerFrame;
  const uint32_t bytes_per_frame = tap->device_format.mBytesPerFrame;
  if (bytes_per_frame == 0) {
    return noErr;
  }

  const uint32_t frame_count = buf->mDataByteSize / bytes_per_frame;
  const uint32_t sample_count = frame_count * channels;
  int16_t *pcm = (int16_t *)malloc(sample_count * sizeof(int16_t));
  if (!pcm) {
    return noErr;
  }

  if (tap->device_format.mFormatFlags & kAudioFormatFlagIsFloat) {
    const float *src = (const float *)buf->mData;
    for (uint32_t i = 0; i < sample_count; ++i) {
      float s = src[i];
      if (s > 1.0f) {
        s = 1.0f;
      } else if (s < -1.0f) {
        s = -1.0f;
      }
      pcm[i] = (int16_t)(s * 32767.0f);
    }
  } else if ((tap->device_format.mFormatFlags & kAudioFormatFlagIsSignedInteger) &&
             tap->device_format.mBitsPerChannel == 16) {
    memcpy(pcm, buf->mData, buf->mDataByteSize);
  } else {
    free(pcm);
    return noErr;
  }

  tap->callback(pcm, sample_count, rate, channels, tap->userdata);
  free(pcm);
  return noErr;
}

ThemisTap *themis_tap_create_system(void) {
  if (@available(macOS 14.2, *)) {
  } else {
    return NULL;
  }

  ThemisTapState *tap = (ThemisTapState *)calloc(1, sizeof(ThemisTapState));
  if (!tap) {
    return NULL;
  }
  tap->tap_id = kAudioObjectUnknown;
  tap->aggregate_id = kAudioObjectUnknown;
  atomic_init(&tap->running, false);

  @autoreleasepool {
    NSArray<NSNumber *> *empty = @[];
    CATapDescription *desc =
        [[CATapDescription alloc] initStereoGlobalTapButExcludeProcesses:empty];
    [desc setName:@"Themis"];
    [desc setPrivate:YES];

    OSStatus st = AudioHardwareCreateProcessTap(desc, &tap->tap_id);
    if (st != noErr) {
      SetDetail(tap, "AudioHardwareCreateProcessTap failed");
      free(tap);
      return NULL;
    }

    NSString *uid = [[NSUUID UUID] UUIDString];
    NSDictionary *agg_desc = @{
      @"name" : @"Themis Tap",
      @"uid" : uid,
      @"subdevices" : @[],
      @"private" : @YES,
      @"stacked" : @NO,
    };

    st = AudioHardwareCreateAggregateDevice((__bridge CFDictionaryRef)agg_desc,
                                            &tap->aggregate_id);
    if (st != noErr) {
      AudioHardwareDestroyProcessTap(tap->tap_id);
      SetDetail(tap, "AudioHardwareCreateAggregateDevice failed");
      free(tap);
      return NULL;
    }

    CFStringRef tap_uid = NULL;
    st = GetTapUid(tap->tap_id, &tap_uid);
    if (st != noErr || !tap_uid) {
      AudioHardwareDestroyAggregateDevice(tap->aggregate_id);
      AudioHardwareDestroyProcessTap(tap->tap_id);
      SetDetail(tap, "kAudioTapPropertyUID failed");
      free(tap);
      return NULL;
    }

    AudioObjectPropertyAddress tap_list =
        Prop(kAudioAggregateDevicePropertyTapList);
    CFArrayRef list =
        CFArrayCreate(NULL, (const void **)&tap_uid, 1, &kCFTypeArrayCallBacks);
    st = AudioObjectSetPropertyData(tap->aggregate_id, &tap_list, 0, NULL,
                                    sizeof(CFArrayRef), &list);
    CFRelease(list);
    CFRelease(tap_uid);

    if (st != noErr) {
      AudioHardwareDestroyAggregateDevice(tap->aggregate_id);
      AudioHardwareDestroyProcessTap(tap->tap_id);
      SetDetail(tap, "kAudioAggregateDevicePropertyTapList failed");
      free(tap);
      return NULL;
    }

    AudioObjectPropertyAddress stream_fmt = Prop(kAudioDevicePropertyStreamFormat);
    stream_fmt.mScope = kAudioObjectPropertyScopeInput;
    UInt32 stream_size = sizeof(AudioStreamBasicDescription);
    OSStatus fmt_st =
        AudioObjectGetPropertyData(tap->aggregate_id, &stream_fmt, 0, NULL,
                                   &stream_size, &tap->device_format);
    if (fmt_st != noErr) {
      tap->device_format.mSampleRate = 48000;
      tap->device_format.mChannelsPerFrame = 2;
      tap->device_format.mBitsPerChannel = 32;
      tap->device_format.mFormatFlags = kAudioFormatFlagIsFloat;
      tap->device_format.mBytesPerFrame = 8;
      tap->device_format.mFramesPerPacket = 1;
      tap->device_format.mBytesPerPacket = 8;
    }

    SetDetail(tap, "Core Audio process tap (system stereo, no BlackHole)");
  }
  return tap;
}

int themis_tap_start(ThemisTap *tap, ThemisTapCallback callback, void *userdata) {
  if (!tap || tap->aggregate_id == kAudioObjectUnknown) {
    return -1;
  }
  tap->callback = callback;
  tap->userdata = userdata;

  OSStatus st =
      AudioDeviceCreateIOProcID(tap->aggregate_id, IOProc, tap, &tap->io_proc);
  if (st != noErr) {
    snprintf(tap->detail, sizeof(tap->detail), "AudioDeviceCreateIOProcID: %d", (int)st);
    return (int)st;
  }
  st = AudioDeviceStart(tap->aggregate_id, tap->io_proc);
  if (st != noErr) {
    AudioDeviceDestroyIOProcID(tap->aggregate_id, tap->io_proc);
    tap->io_proc = NULL;
    snprintf(tap->detail, sizeof(tap->detail),
             "AudioDeviceStart: %d — allow System Audio Recording", (int)st);
    return (int)st;
  }
  atomic_store(&tap->running, true);
  return 0;
}

void themis_tap_stop(ThemisTap *tap) {
  if (!tap) {
    return;
  }
  atomic_store(&tap->running, false);
  if (tap->io_proc) {
    AudioDeviceStop(tap->aggregate_id, tap->io_proc);
    AudioDeviceDestroyIOProcID(tap->aggregate_id, tap->io_proc);
    tap->io_proc = NULL;
  }
}

void themis_tap_destroy(ThemisTap *tap) {
  if (!tap) {
    return;
  }
  themis_tap_stop(tap);
  if (tap->aggregate_id != kAudioObjectUnknown) {
    AudioHardwareDestroyAggregateDevice(tap->aggregate_id);
  }
  if (tap->tap_id != kAudioObjectUnknown) {
    AudioHardwareDestroyProcessTap(tap->tap_id);
  }
  free(tap);
}

const char *themis_tap_detail(const ThemisTap *tap) {
  if (!tap) {
    return "";
  }
  return tap->detail;
}

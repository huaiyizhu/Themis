#pragma once

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct ThemisTapState ThemisTap;

/// Called from Core Audio IO thread with interleaved PCM16 samples.
typedef void (*ThemisTapCallback)(
    const int16_t *samples,
    uint32_t num_samples,
    uint32_t sample_rate,
    uint16_t channels,
    void *userdata);

/// Create a system-wide stereo process tap (macOS 14.2+). Returns NULL on failure.
ThemisTap *themis_tap_create_system(void);

/// Register callback and start IO on the aggregate device.
int themis_tap_start(ThemisTap *tap, ThemisTapCallback callback, void *userdata);

void themis_tap_stop(ThemisTap *tap);

void themis_tap_destroy(ThemisTap *tap);

/// Human-readable status (device name / error), valid until destroy.
const char *themis_tap_detail(const ThemisTap *tap);

#ifdef __cplusplus
}
#endif

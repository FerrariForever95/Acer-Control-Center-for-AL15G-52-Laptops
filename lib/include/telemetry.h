#ifndef ACERCONTROL_TELEMETRY_H
#define ACERCONTROL_TELEMETRY_H

#include "context.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Fan duty cycle, 0-100 %. */
int acer_cpu_fan_percent(AcerContext *ctx, int *percent);
int acer_gpu_fan_percent(AcerContext *ctx, int *percent);
int acer_system_fan_percent(AcerContext *ctx, int *percent);

/*
 * Deprecated: despite the name these return a duty-cycle percentage, not
 * RPM. Kept for ABI compatibility; use the *_percent functions.
 */
int acer_cpu_fan_rpm(AcerContext *ctx, int *rpm);
int acer_gpu_fan_rpm(AcerContext *ctx, int *rpm);
int acer_system_fan_rpm(AcerContext *ctx, int *rpm);

#ifdef __cplusplus
}
#endif

#endif

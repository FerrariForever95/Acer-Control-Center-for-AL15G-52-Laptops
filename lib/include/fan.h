#ifndef ACERCONTROL_FAN_H
#define ACERCONTROL_FAN_H

#include "context.h"

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Fan control.
 *
 * Percentages are 0-100. The library only validates the range; safety
 * policy (minimum speed, thermal cut-out) belongs to the caller, which in
 * this project is acer-control-daemon.
 */

/*
 * Pin the CPU and GPU fans to fixed speeds. The system fan (fan 3) is left
 * at whatever the firmware is currently running it at.
 *
 * Returns 0 on success, negative errno on failure.
 */
int acer_fan_set_manual(AcerContext *ctx, int cpu_percent, int gpu_percent);

/*
 * Hand all fans back to the firmware's automatic control.
 *
 * Returns 0 on success, negative errno on failure.
 */
int acer_fan_set_auto(AcerContext *ctx);

#ifdef __cplusplus
}
#endif

#endif /* ACERCONTROL_FAN_H */

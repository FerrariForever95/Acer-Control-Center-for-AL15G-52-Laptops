#include <errno.h>
#include <stdint.h>

#include "../include/fan.h"
#include "../include/ioctl.h"
#include "acer_io_ioctl.h"

/*
 * Argument layout for W_CL_FANSPEED (ACER_CMD_SET_FANSPEED_VALUE):
 * one PWM byte (0-255) per fan, fan 1 in bits 0-7, fan 2 in 8-15,
 * fan 3 in 16-23.
 *
 * Argument for W_CL_FANAUTO is a bitmask of fans to return to automatic.
 * NOTE: the mask value is inherited from the Clevo EC protocol and has not
 * been verified on every model. Set ACER_FAN_AUTO_MASK if your hardware
 * needs a different value.
 */
#ifndef ACER_FAN_AUTO_MASK
#define ACER_FAN_AUTO_MASK 0x0F
#endif

static int percent_to_pwm(int percent)
{
    return (percent * 255 + 50) / 100;
}

int acer_fan_set_manual(AcerContext *ctx, int cpu_percent, int gpu_percent)
{
    int32_t sys_raw = 0;
    int32_t arg;

    if (!ctx)
        return -EINVAL;

    if (cpu_percent < 0 || cpu_percent > 100 ||
        gpu_percent < 0 || gpu_percent > 100)
        return -EINVAL;

    /* Preserve the system fan so we never stop it by accident. */
    if (acer_ioctl_read(ctx, R_CL_FANINFO3, &sys_raw) < 0)
        return -errno;

    arg = (int32_t)(percent_to_pwm(cpu_percent) |
                    (percent_to_pwm(gpu_percent) << 8) |
                    ((sys_raw & 0xff) << 16));

    if (acer_ioctl_write(ctx, W_CL_FANSPEED, &arg) < 0)
        return -errno;

    return 0;
}

int acer_fan_set_auto(AcerContext *ctx)
{
    int32_t arg = ACER_FAN_AUTO_MASK;

    if (!ctx)
        return -EINVAL;

    if (acer_ioctl_write(ctx, W_CL_FANAUTO, &arg) < 0)
        return -errno;

    return 0;
}

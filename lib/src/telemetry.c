#include <errno.h>
#include <stdint.h>

#include "../include/ioctl.h"
#include "../include/telemetry.h"
#include "acer_io_ioctl.h"

static int read_faninfo(AcerContext *ctx, unsigned long request, int *faninfo)
{
    int32_t raw = 0;

    if (!ctx || !faninfo)
        return -EINVAL;

    if (acer_ioctl_read(ctx, request, &raw) < 0)
        return -errno;

    *faninfo = raw;
    return 0;
}

static int faninfo_to_percent(int faninfo)
{
    int pwm = faninfo & 0xff;

    return (pwm * 100 + 127) / 255;
}

int acer_cpu_fan_percent(AcerContext *ctx, int *percent)
{
    int faninfo;
    int ret = read_faninfo(ctx, R_CL_FANINFO1, &faninfo);

    if (ret < 0)
        return ret;

    *percent = faninfo_to_percent(faninfo);

    return 0;
}

int acer_gpu_fan_percent(AcerContext *ctx, int *percent)
{
    int faninfo;
    int ret = read_faninfo(ctx, R_CL_FANINFO2, &faninfo);

    if (ret < 0)
        return ret;

    *percent = faninfo_to_percent(faninfo);

    return 0;
}

int acer_system_fan_percent(AcerContext *ctx, int *percent)
{
    int faninfo;
    int ret = read_faninfo(ctx, R_CL_FANINFO3, &faninfo);

    if (ret < 0)
        return ret;

    *percent = faninfo_to_percent(faninfo);

    return 0;
}

int acer_cpu_fan_rpm(AcerContext *ctx, int *rpm)
{
    return acer_cpu_fan_percent(ctx, rpm);
}

int acer_gpu_fan_rpm(AcerContext *ctx, int *rpm)
{
    return acer_gpu_fan_percent(ctx, rpm);
}

int acer_system_fan_rpm(AcerContext *ctx, int *rpm)
{
    return acer_system_fan_percent(ctx, rpm);
}

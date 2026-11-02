#include <errno.h>
#include <sys/ioctl.h>

#include "../include/context.h"
#include "internal.h"
#include "../include/ioctl.h"

int acer_ioctl_read(AcerContext *ctx, unsigned long request, void *data)
{
    if (!ctx)
        return -EINVAL;

    return ioctl(ctx->fd, request, data);
}

int acer_ioctl_write(AcerContext *ctx, unsigned long request, void *data)
{
    if (!ctx)
        return -EINVAL;

    return ioctl(ctx->fd, request, data);
}

int acer_ioctl_command(AcerContext *ctx, unsigned long request)
{
    if (!ctx)
        return -EINVAL;

    return ioctl(ctx->fd, request);
}

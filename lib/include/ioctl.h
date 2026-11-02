#ifndef LIBACERCONTROL_IOCTL_H
#define LIBACERCONTROL_IOCTL_H

#include <stdint.h>

typedef struct AcerContext AcerContext;

int acer_ioctl_read(AcerContext *ctx, unsigned long request, void *data);

int acer_ioctl_write(AcerContext *ctx, unsigned long request, void *data);

int acer_ioctl_command(AcerContext *ctx, unsigned long request);

#endif

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "../include/context.h"
#include "internal.h"

AcerContext *acer_open(void)
{
    AcerContext *ctx = malloc(sizeof(*ctx));

    if (!ctx) {
        perror("malloc");
        return NULL;
    }

    ctx->fd = open(ACERCONTROL_DEVICE, O_RDWR);

    if (ctx->fd < 0) {
        fprintf(stderr,
                "Failed to open %s: %s\n",
                ACERCONTROL_DEVICE,
                strerror(errno));
        free(ctx);
        return NULL;
    }

    return ctx;
}

void acer_close(AcerContext *ctx)
{
    if (!ctx)
        return;

    if (ctx->fd >= 0)
        close(ctx->fd);

    free(ctx);
}

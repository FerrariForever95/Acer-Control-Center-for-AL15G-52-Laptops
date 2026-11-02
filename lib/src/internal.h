#ifndef ACERCONTROL_INTERNAL_H
#define ACERCONTROL_INTERNAL_H

#include <fcntl.h>
#include <unistd.h>

#pragma once

#include <stdint.h>

struct AcerContext {
    int fd;

    uint32_t abi_major;
    uint32_t abi_minor;
};

#define ACERCONTROL_DEVICE "/dev/acer_io"

#endif /* ACERCONTROL_INTERNAL_H */

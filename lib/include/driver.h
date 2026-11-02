#ifndef ACERCONTROL_DRIVER_H
#define ACERCONTROL_DRIVER_H

#include "context.h"

#ifdef __cplusplus
extern "C" {
#endif

int acer_driver_version(
    AcerContext *ctx,
    char *buffer,
    unsigned int size);

int acer_driver_hardware_check(
    AcerContext *ctx,
    int *supported);

int acer_driver_interface(
    AcerContext *ctx,
    char *buffer,
    unsigned int size);

int acer_driver_abi_major(void);

int acer_driver_abi_minor(void);

#ifdef __cplusplus
}
#endif

#endif

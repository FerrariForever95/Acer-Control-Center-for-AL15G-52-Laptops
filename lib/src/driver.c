#include <errno.h>
#include <string.h>

#include "../include/driver.h"
#include "../include/ioctl.h"
#include "../driver/include/acer_io_ioctl.h"

int acer_driver_version(
    AcerContext *ctx,
    char *buffer,
    unsigned int size)
{
    if (!ctx || !buffer || size == 0)
        return -EINVAL;

    memset(buffer, 0, size);

    if (acer_ioctl_read(ctx, R_MOD_VERSION, buffer) < 0)
        return -errno;

    return 0;
}

int acer_driver_hardware_check(
    AcerContext *ctx,
    int *supported)
{
    if (!ctx || !supported)
        return -EINVAL;

    *supported = 0;

    if (acer_ioctl_read(ctx, R_HWCHECK_CL, supported) < 0)
        return -errno;

    return 0;
}

int acer_driver_interface(
    AcerContext *ctx,
    char *buffer,
    unsigned int size)
{
    if (!ctx || !buffer || size == 0)
        return -EINVAL;

    memset(buffer, 0, size);

    if (acer_ioctl_read(ctx, R_CL_HW_IF_STR, buffer) < 0)
        return -errno;

    return 0;
}

int acer_driver_abi_major(void)
{
    return ACER_IO_ABI_MAJOR;
}

int acer_driver_abi_minor(void)
{
    return ACER_IO_ABI_MINOR;
}

#include <stdio.h>

#include <acercontrol.h>

int main(void)
{
    AcerContext *ctx = acer_open();

    if (!ctx) {
        puts("open failed");
        return 1;
    }

    char version[32];
    int hw;

    if (!acer_driver_version(ctx, version, sizeof(version)))
        printf("Driver: %s\n", version);

    if (!acer_driver_hardware_check(ctx, &hw))
        printf("Supported: %d\n", hw);

    acer_close(ctx);

    return 0;
}

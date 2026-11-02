project(
    'libacercontrol',
    'c',
    version: '0.1.0',
    default_options: ['warning_level=3'],
)

inc = include_directories(
    'include',
    '../driver/include',
)

src = files(
    'src/acercontrol.c',
    'src/context.c',
    'src/driver.c',
    'src/fan.c',
    'src/ioctl.c',
    'src/telemetry.c',
)

libacercontrol = library(
    'acercontrol',
    src,
    include_directories: inc,
    install: true,
    version: meson.project_version(),
    soversion: '0',
)

install_headers(
    'include/acercontrol.h',
    'include/context.h',
    'include/version.h',
    'include/driver.h',
    'include/fan.h',
    'include/ioctl.h',
    'include/telemetry.h',
    subdir: 'acercontrol',
)

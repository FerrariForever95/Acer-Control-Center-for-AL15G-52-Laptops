use std::os::raw::c_int;

#[repr(C)]
pub struct AcerContext {
    _private: [u8; 0],
}

#[link(name = "acercontrol")]
unsafe extern "C" {
    pub fn acer_open() -> *mut AcerContext;
    pub fn acer_close(ctx: *mut AcerContext);

    pub fn acer_fan_set_manual(
        ctx: *mut AcerContext,
        cpu_percent: c_int,
        gpu_percent: c_int,
    ) -> c_int;

    pub fn acer_fan_set_auto(ctx: *mut AcerContext) -> c_int;
}

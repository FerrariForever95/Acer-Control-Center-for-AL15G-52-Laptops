use crate::libacercontrol::ffi;

pub struct Context {
    ptr: *mut ffi::AcerContext,
}

impl Context {
    pub fn new() -> Result<Self, &'static str> {
        let ptr = unsafe { ffi::acer_open() };

        if ptr.is_null() {
            Err("Failed to open Acer driver")
        } else {
            Ok(Self { ptr })
        }
    }

    pub fn as_ptr(&self) -> *mut ffi::AcerContext {
        self.ptr
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                ffi::acer_close(self.ptr);
            }
        }
    }
}

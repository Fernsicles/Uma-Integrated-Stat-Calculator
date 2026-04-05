use std::ffi::{c_char, c_void};

#[repr(C)]
pub struct Vtable {
    // Function pointers (see API Reference below)
}

#[repr(i32)]
pub enum InitResult {
    Error = 0,
    Ok = 1,
}

static mut VTABLE: Option<&'static Vtable> = None;

#[unsafe(export_name = "hachimi_init")]
pub extern "C" fn hachimi_init(vtable: *const Vtable, version: i32) -> InitResult {
    if vtable.is_null() {
        return InitResult::Error;
    }
    if version < 2 {
        return InitResult::Error;
    }

    unsafe {
        VTABLE = Some(&*vtable);
    }

    InitResult::Ok
}

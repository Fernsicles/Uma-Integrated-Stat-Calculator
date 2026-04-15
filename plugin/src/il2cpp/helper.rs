use std::{
    ffi::{CStr, CString, c_char, c_void},
    ptr::{null, null_mut},
};

use crate::{il2cpp::types::*, plugin_api::Vtable};

pub type GetU8 = unsafe extern "C" fn(*mut Il2CppObject, *const c_void) -> u8;
pub type GetI32 = unsafe extern "C" fn(*mut Il2CppObject, *const c_void) -> i32;
pub type GetPointer = unsafe extern "C" fn(*mut Il2CppObject, *const c_void) -> *mut c_void;
pub type GetObject = unsafe extern "C" fn(*mut Il2CppObject, *const c_void) -> *mut Il2CppObject;

static mut VTABLE: Option<&'static Vtable> = None;

pub unsafe fn log(log_level: i32, log_str: &str) {
    unsafe {
        (VTABLE.unwrap().log)(
            log_level,
            CString::new("UISC").unwrap().as_ptr() as *const c_char,
            CString::new(log_str).unwrap().as_ptr() as *const c_char,
        );
    }
}

pub fn init(vtable: &'static Vtable) {
    unsafe {
        VTABLE = Some(vtable);
    }
}

pub fn array_get_byte(array: &Il2CppArray, index: usize) -> u8 {
    unsafe {
        let data_ptr = (array as *const _ as *const u8).add(32) as *const u8;
        *data_ptr.add(index)
    }
}

pub fn array_get_obj(array: &Il2CppArray, index: usize) -> *mut Il2CppObject {
    unsafe {
        let data_ptr = (array as *const _ as *const u8).add(32) as *const *mut Il2CppObject;
        *data_ptr.add(index)
    }
}

pub fn array_get_int(array: &Il2CppArray, index: usize) -> i32 {
    unsafe {
        let data_ptr = (array as *const _ as *const u8).add(32) as *const i32;
        *data_ptr.add(index)
    }
}

pub fn il2cppstring_as_string(string: &Il2CppString) -> String {
    let slice =
        unsafe { std::slice::from_raw_parts(string.chars.as_ptr(), string.length as usize) };
    return String::from_utf16_lossy(slice);
}

pub fn resolve_icall(name: &str) -> Il2CppMethodPointer {
    unsafe {
        let vtable = VTABLE.unwrap();
        let icall = (vtable.il2cpp_resolve_icall)(CString::new(name).unwrap().as_ptr());
        return icall;
    }
}

pub fn get_singleton(class: *mut Il2CppClass) -> *mut Il2CppObject {
    unsafe {
        let vtable = VTABLE.unwrap();
        return (vtable.il2cpp_get_singleton_like_instance)(class);
    }
}

pub fn get_class_from_namespace(image: &str, namespace: &str, classname: &str) -> *mut Il2CppClass {
    unsafe {
        let vtable = VTABLE.unwrap();
        let image = (vtable.il2cpp_get_assembly_image)(CString::new(image).unwrap().as_ptr());
        return (vtable.il2cpp_get_class)(
            image,
            CString::new(namespace).unwrap().as_ptr(),
            CString::new(classname).unwrap().as_ptr(),
        );
    }
}

pub fn get_class_from_image(image: &str, path: &str) -> *mut Il2CppClass {
    let path_parts = path.split('.');
    let mut path_parts: Vec<&str> = path_parts.collect();
    unsafe {
        let vtable = VTABLE.unwrap();
        let image = (vtable.il2cpp_get_assembly_image)(CString::new(image).unwrap().as_ptr());
        let namespace = path_parts.remove(0);
        let mut class = (vtable.il2cpp_get_class)(
            image,
            CString::new(namespace).unwrap().as_ptr(),
            CString::new(path_parts.remove(0)).unwrap().as_ptr(),
        );
        for part in path_parts.iter() {
            class = (vtable.il2cpp_find_nested_class)(class, CString::new(*part).unwrap().as_ptr());
        }
        return class;
    }
}

pub fn get_class(path: &str) -> *mut Il2CppClass {
    return get_class_from_image("umamusume", path);
}

pub fn get_gallop_class(class_name: &str) -> *mut Il2CppClass {
    return get_class(("Gallop.".to_owned() + class_name).as_str());
}

pub fn get_method(class: *mut Il2CppClass, name: &str, num_args: i32) -> *mut c_void {
    unsafe {
        let vtable = VTABLE.unwrap();
        return (vtable.il2cpp_get_method_addr)(
            class,
            CString::new(name).unwrap().as_ptr(),
            num_args,
        );
    }
}

pub fn get_nth_method(class: *mut Il2CppClass, name: &str, n: usize) -> usize {
    unsafe {
        let vtable = VTABLE.unwrap();

        let mut iter: *mut c_void = null_mut();

        let mut count = 0;
        loop {
            let method = (vtable.il2cpp_class_get_methods)(class, &mut iter);
            if method.is_null() {
                break;
            }

            // Check name
            let method_name = CStr::from_ptr((*method).name);
            if method_name.to_str().unwrap() != name {
                continue;
            }
            count += 1;

            if count == n {
                return (*method).methodPointer;
            }
        }

        return 0;
    }
}

pub fn get_method_overload(
    class: *mut Il2CppClass,
    name: &str,
    params: *const Il2CppTypeEnum,
    param_count: usize,
) -> *mut c_void {
    unsafe {
        let vtable = VTABLE.unwrap();
        return (vtable.il2cpp_get_method_overload_addr)(
            class,
            CString::new(name).unwrap().as_ptr(),
            params,
            param_count,
        );
    }
}

pub fn get_i32(class: *mut Il2CppClass, property: &str, this: *mut Il2CppObject) -> i32 {
    unsafe {
        let getter_name = format!("get_{property}");
        let getter: GetI32 = std::mem::transmute(get_method(class, getter_name.as_str(), 0));
        return getter(this, null());
    }
}

pub fn get_u8(class: *mut Il2CppClass, property: &str, this: *mut Il2CppObject) -> u8 {
    unsafe {
        let getter_name = CString::new(format!("get_{property}")).unwrap();
        let vtable = VTABLE.unwrap();
        let getter: GetU8 = std::mem::transmute((vtable.il2cpp_get_method_addr)(
            class,
            getter_name.as_ptr(),
            0,
        ));
        return getter(this, null());
    }
}

pub fn get_pointer(
    class: *mut Il2CppClass,
    property: &str,
    this: *mut Il2CppObject,
) -> *const c_void {
    unsafe {
        let getter_name = CString::new(format!("get_{property}")).unwrap();
        let vtable = VTABLE.unwrap();
        let getter: GetPointer = std::mem::transmute((vtable.il2cpp_get_method_addr)(
            class,
            getter_name.as_ptr(),
            0,
        ));
        return getter(this, null());
    }
}

pub fn get_object(
    class: *mut Il2CppClass,
    property: &str,
    this: *mut Il2CppObject,
) -> *mut Il2CppObject {
    return get_pointer(class, property, this) as *mut Il2CppObject;
}

pub fn get_i32_field(class: *mut Il2CppClass, field: &str, this: *const Il2CppObject) -> i32 {
    unsafe {
        let vtable = VTABLE.unwrap();
        let field =
            (vtable.il2cpp_get_field_from_name)(class, CString::new(field).unwrap().as_ptr());
        let mut value = 0;
        (vtable.il2cpp_get_field_value)(
            this as *mut Il2CppObject,
            field,
            &mut value as *mut _ as _,
        );
        return value;
    }
}

pub fn get_pointer_field(
    class: *mut Il2CppClass,
    field: &str,
    this: *const Il2CppObject,
) -> *mut c_void {
    unsafe {
        let vtable = VTABLE.unwrap();
        let field =
            (vtable.il2cpp_get_field_from_name)(class, CString::new(field).unwrap().as_ptr());
        let mut value = null_mut();
        (vtable.il2cpp_get_field_value)(
            this as *mut Il2CppObject,
            field,
            &mut value as *mut _ as _,
        );
        return value;
    }
}

pub fn get_object_field(
    class: *mut Il2CppClass,
    field: &str,
    this: *const Il2CppObject,
) -> *mut Il2CppObject {
    return get_pointer_field(class, field, this) as *mut Il2CppObject;
}

pub fn object_new(class: *mut Il2CppClass) -> *mut Il2CppObject {
    unsafe {
        let vtable = VTABLE.unwrap();
        return (vtable.il2cpp_object_new)(class);
    }
}

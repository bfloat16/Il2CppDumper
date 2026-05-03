use std::ffi::{c_char, c_void};

#[repr(C)]
pub struct Il2CppClass;

#[repr(C)]
pub struct Il2CppType;

#[repr(C)]
pub struct MethodInfo {
    pub klass: *const Il2CppClass,
    pub method_pointer: *const c_void,
    _pad: [u8; 0x20],
    pub flags: u16,
}

#[repr(C)]
pub struct FieldInfo;

#[repr(C)]
pub struct PropertyInfo;

#[repr(C)]
pub struct EventInfo {
    pub name: *const c_char,
    pub event_type: *const Il2CppType,
    pub parent: *const Il2CppClass,
    pub add: *const MethodInfo,
    pub remove: *const MethodInfo,
    pub raise: *const MethodInfo,
    pub token: u32,
}

#[repr(C)]
pub struct Il2CppAssembly;

#[repr(C)]
pub struct Il2CppDomain;

#[repr(C)]
pub struct Il2CppImage;

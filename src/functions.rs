use log::{debug, info};
use std::ffi::{c_char, c_void};
use std::fmt::Write;

use crate::constants::{FUNCTION_TABLE_OFFSET, METHOD_GET_PARAM_NAME_RVA};
use crate::module::FunctionPtr;
use crate::types::*;
use crate::utils::format_rva;

const TABLE_SIZE: usize = 512;

macro_rules! named_indices {
    ($($index:expr => $name:ident),* $(,)?) => {
        fn index_name(i: usize) -> Option<&'static str> {
            match i {
                $($index => Some(stringify!($name)),)*
                _ => None,
            }
        }
    };
}

macro_rules! pick {
    ($table:expr, $index:expr) => {
        if $table[$index].is_null() {
            None
        } else {
            Some(FunctionPtr::new($table[$index]))
        }
    };
}

named_indices! {
    22 => il2cpp_assembly_get_image,
    24 => il2cpp_class_is_generic,
    25 => il2cpp_class_is_inflated,
    31 => il2cpp_class_get_fields,
    33 => il2cpp_class_get_interfaces,
    35 => il2cpp_class_get_methods,
    37 => il2cpp_class_get_name,
    39 => il2cpp_class_get_namespace,
    40 => il2cpp_class_get_parent,
    41 => il2cpp_class_get_declaring_type,
    42 => il2cpp_class_instance_size,
    43 => il2cpp_class_is_valuetype,
    45 => il2cpp_class_get_flags,
    49 => il2cpp_class_from_type,
    51 => il2cpp_class_get_type,
    52 => il2cpp_field_get_value,
    53 => il2cpp_class_is_enum,
    54 => il2cpp_class_get_image,
    55 => il2cpp_field_static_get_value,
    62 => il2cpp_domain_get,
    64 => il2cpp_domain_get_assemblies,
    71 => il2cpp_field_get_flags,
    72 => il2cpp_field_get_name,
    74 => il2cpp_field_get_offset,
    75 => il2cpp_field_get_type,
    113 => il2cpp_method_get_return_type,
    114 => il2cpp_method_get_name,
    117 => il2cpp_method_is_generic,
    118 => il2cpp_method_is_inflated,
    119 => il2cpp_method_is_instance,
    120 => il2cpp_method_get_param_count,
    121 => il2cpp_method_get_param,
    157 => il2cpp_type_get_type,
    158 => il2cpp_type_get_class_or_element_class,
    159 => il2cpp_type_get_name,
    160 => il2cpp_type_is_byref,
    161 => il2cpp_type_get_attrs,
    174 => il2cpp_image_get_name,
    177 => il2cpp_image_get_class_count,
    181 => il2cpp_image_get_class,
}

#[derive(Clone)]
pub struct Il2CppFunctions {
    pub il2cpp_assembly_get_image: Option<FunctionPtr<fn(*const Il2CppAssembly) -> *const Il2CppImage>>,

    pub il2cpp_class_get_fields: Option<FunctionPtr<fn(*const Il2CppClass, *const *const c_void) -> *const FieldInfo>>,
    pub il2cpp_class_get_interfaces: Option<FunctionPtr<fn(*const Il2CppClass, *const *const c_void) -> *const Il2CppClass>>,
    pub il2cpp_class_get_methods: Option<FunctionPtr<fn(*const Il2CppClass, *const *const c_void) -> *const MethodInfo>>,
    pub il2cpp_class_get_name: Option<FunctionPtr<fn(*const Il2CppClass) -> *const c_char>>,
    pub il2cpp_class_get_namespace: Option<FunctionPtr<fn(*const Il2CppClass) -> *const c_char>>,
    pub il2cpp_class_get_parent: Option<FunctionPtr<fn(*const Il2CppClass) -> *const Il2CppClass>>,
    pub il2cpp_class_instance_size: Option<FunctionPtr<fn(*const Il2CppClass) -> u32>>,
    pub il2cpp_class_is_valuetype: Option<FunctionPtr<fn(*const Il2CppClass) -> bool>>,
    pub il2cpp_class_get_flags: Option<FunctionPtr<fn(*const Il2CppClass) -> i32>>,
    pub il2cpp_class_from_type: Option<FunctionPtr<fn(*const Il2CppType) -> *const Il2CppClass>>,
    pub il2cpp_class_is_enum: Option<FunctionPtr<fn(*const Il2CppClass) -> bool>>,

    pub il2cpp_domain_get: Option<FunctionPtr<fn() -> *const Il2CppDomain>>,
    pub il2cpp_domain_get_assemblies: Option<FunctionPtr<fn(*const Il2CppDomain, *const usize) -> *const *const Il2CppAssembly>>,

    pub il2cpp_field_get_flags: Option<FunctionPtr<fn(*const FieldInfo) -> i32>>,
    pub il2cpp_field_get_name: Option<FunctionPtr<fn(*const FieldInfo) -> *const c_char>>,
    pub il2cpp_field_get_offset: Option<FunctionPtr<fn(*const FieldInfo) -> usize>>,
    pub il2cpp_field_get_type: Option<FunctionPtr<fn(*const FieldInfo) -> *const Il2CppType>>,

    pub il2cpp_method_get_return_type: Option<FunctionPtr<fn(*const MethodInfo) -> *const Il2CppType>>,
    pub il2cpp_method_get_name: Option<FunctionPtr<fn(*const MethodInfo) -> *const c_char>>,
    pub il2cpp_method_get_param_count: Option<FunctionPtr<fn(*const MethodInfo) -> u32>>,
    pub il2cpp_method_get_param: Option<FunctionPtr<fn(*const MethodInfo, u32) -> *const Il2CppType>>,

    pub il2cpp_type_get_name: Option<FunctionPtr<fn(*const Il2CppType) -> *const c_char>>,
    pub il2cpp_type_is_byref: Option<FunctionPtr<fn(*const Il2CppType) -> bool>>,
    pub il2cpp_type_get_attrs: Option<FunctionPtr<fn(*const Il2CppType) -> u32>>,

    pub il2cpp_image_get_name: Option<FunctionPtr<fn(*const Il2CppImage) -> *const c_char>>,
    pub il2cpp_image_get_class_count: Option<FunctionPtr<fn(*const Il2CppImage) -> usize>>,
    pub il2cpp_image_get_class: Option<FunctionPtr<fn(*const Il2CppImage, usize) -> *const Il2CppClass>>,

    pub il2cpp_type_get_type: Option<FunctionPtr<fn(*const Il2CppType) -> u32>>,
    pub il2cpp_type_get_class_or_element_class: Option<FunctionPtr<fn(*const Il2CppType) -> *const Il2CppClass>>,
    pub il2cpp_class_get_type: Option<FunctionPtr<fn(*const Il2CppClass) -> *const Il2CppType>>,
    pub il2cpp_class_get_image: Option<FunctionPtr<fn(*const Il2CppClass) -> *const Il2CppImage>>,
    pub il2cpp_class_get_declaring_type: Option<FunctionPtr<fn(*const Il2CppClass) -> *const Il2CppClass>>,
    pub il2cpp_class_is_generic: Option<FunctionPtr<fn(*const Il2CppClass) -> bool>>,
    pub il2cpp_class_is_inflated: Option<FunctionPtr<fn(*const Il2CppClass) -> bool>>,
    pub il2cpp_class_num_fields: Option<FunctionPtr<fn(*const Il2CppClass) -> usize>>,

    pub il2cpp_method_get_param_name: Option<FunctionPtr<fn(*const MethodInfo, u32) -> *const c_char>>,
    pub il2cpp_method_get_flags: Option<FunctionPtr<fn(*const MethodInfo, *mut u32) -> u32>>,
    pub il2cpp_method_get_token: Option<FunctionPtr<fn(*const MethodInfo) -> u32>>,
    pub il2cpp_method_is_generic: Option<FunctionPtr<fn(*const MethodInfo) -> bool>>,
    pub il2cpp_method_is_inflated: Option<FunctionPtr<fn(*const MethodInfo) -> bool>>,
    pub il2cpp_method_is_instance: Option<FunctionPtr<fn(*const MethodInfo) -> bool>>,

    pub il2cpp_class_get_properties: Option<FunctionPtr<fn(*const Il2CppClass, *const *const c_void) -> *const PropertyInfo>>,
    pub il2cpp_property_get_name: Option<FunctionPtr<fn(*const PropertyInfo) -> *const c_char>>,
    pub il2cpp_property_get_get_method: Option<FunctionPtr<fn(*const PropertyInfo) -> *const MethodInfo>>,
    pub il2cpp_property_get_set_method: Option<FunctionPtr<fn(*const PropertyInfo) -> *const MethodInfo>>,
    pub il2cpp_property_get_flags: Option<FunctionPtr<fn(*const PropertyInfo) -> u32>>,
    pub il2cpp_property_get_parent: Option<FunctionPtr<fn(*const PropertyInfo) -> *const Il2CppClass>>,

    pub il2cpp_class_get_events: Option<FunctionPtr<fn(*const Il2CppClass, *const *const c_void) -> *const EventInfo>>,

    pub il2cpp_field_get_value: Option<FunctionPtr<fn(*const c_void, *const FieldInfo, *mut c_void)>>,
    pub il2cpp_field_static_get_value: Option<FunctionPtr<fn(*const FieldInfo, *mut c_void)>>,
}

impl Il2CppFunctions {
    pub fn new(base: usize, modules: &[(&str, &crate::module::Module)]) -> (Self, String) {
        let funcs = (base + FUNCTION_TABLE_OFFSET) as *const *const c_void;
        debug!("function table at 0x{:x}", base + FUNCTION_TABLE_OFFSET);

        let table: Vec<*const c_void> = (0..TABLE_SIZE).map(|i| unsafe { *funcs.add(i) }).collect();

        let ga_base = modules.iter().find(|&&(name, _)| name == "GameAssembly").map(|&(_, m)| m.handle as usize).unwrap_or(0);

        let mut rva_log = String::new();
        let mut resolved = 0usize;
        for (i, &addr) in table.iter().enumerate() {
            if addr.is_null() {
                continue;
            }
            resolved += 1;
            let rva = format_rva(addr as usize, modules);
            match index_name(i) {
                Some(name) => writeln!(rva_log, "[{:>3}] {} = {}", i, name, rva).unwrap(),
                None => writeln!(rva_log, "[{:>3}] (unknown) = {}", i, rva).unwrap(),
            }
        }
        info!("resolved {}/{} function table entries", resolved, TABLE_SIZE);

        let funcs = Il2CppFunctions {
            il2cpp_assembly_get_image: pick!(table, 22),
            il2cpp_class_get_fields: pick!(table, 31),
            il2cpp_class_get_interfaces: pick!(table, 33),
            il2cpp_class_get_methods: pick!(table, 35),
            il2cpp_class_get_name: pick!(table, 37),
            il2cpp_class_get_namespace: pick!(table, 39),
            il2cpp_class_get_parent: pick!(table, 40),
            il2cpp_class_instance_size: pick!(table, 42),
            il2cpp_class_is_valuetype: pick!(table, 43),
            il2cpp_class_get_flags: pick!(table, 45),
            il2cpp_class_from_type: pick!(table, 49),
            il2cpp_class_is_enum: pick!(table, 53),
            il2cpp_domain_get: pick!(table, 62),
            il2cpp_domain_get_assemblies: pick!(table, 64),
            il2cpp_field_get_flags: pick!(table, 71),
            il2cpp_field_get_name: pick!(table, 72),
            il2cpp_field_get_offset: pick!(table, 74),
            il2cpp_field_get_type: pick!(table, 75),
            il2cpp_method_get_return_type: pick!(table, 113),
            il2cpp_method_get_name: pick!(table, 114),
            il2cpp_method_get_param_count: pick!(table, 120),
            il2cpp_method_get_param: pick!(table, 121),
            il2cpp_type_get_name: pick!(table, 159),
            il2cpp_type_is_byref: pick!(table, 160),
            il2cpp_type_get_attrs: pick!(table, 161),
            il2cpp_image_get_name: pick!(table, 174),
            il2cpp_image_get_class_count: pick!(table, 177),
            il2cpp_image_get_class: pick!(table, 181),
            il2cpp_type_get_type: pick!(table, 157),
            il2cpp_type_get_class_or_element_class: pick!(table, 158),
            il2cpp_class_get_type: pick!(table, 51),
            il2cpp_class_get_image: pick!(table, 54),
            il2cpp_class_get_declaring_type: pick!(table, 41),
            il2cpp_class_is_generic: pick!(table, 24),
            il2cpp_class_is_inflated: pick!(table, 25),
            il2cpp_class_num_fields: None,
            il2cpp_method_get_param_name: if ga_base != 0 { Some(FunctionPtr::new((ga_base + METHOD_GET_PARAM_NAME_RVA) as *const c_void)) } else { None },
            il2cpp_method_get_flags: None,
            il2cpp_method_get_token: None,
            il2cpp_method_is_generic: pick!(table, 117),
            il2cpp_method_is_inflated: pick!(table, 118),
            il2cpp_method_is_instance: pick!(table, 119),
            il2cpp_class_get_properties: None,
            il2cpp_property_get_name: None,
            il2cpp_property_get_get_method: None,
            il2cpp_property_get_set_method: None,
            il2cpp_property_get_flags: None,
            il2cpp_property_get_parent: None,
            il2cpp_class_get_events: None,
            il2cpp_field_get_value: pick!(table, 52),
            il2cpp_field_static_get_value: pick!(table, 55),
        };

        (funcs, rva_log)
    }
}

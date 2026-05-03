use log::{debug, error, info};
use std::{
    error::Error,
    ffi::{c_void, CStr},
    path::PathBuf,
    sync::OnceLock,
};
use thiserror::Error;

use crate::{
    functions::Il2CppFunctions,
    module::{Module, ModuleError},
    types::*,
};

macro_rules! get_function {
    ($self:ident, $name:ident) => {{
        $self.functions.$name.as_ref().expect(concat!("function not found: ", stringify!($name)))
    }};
}

macro_rules! cstr_to_string {
    ($cstr:expr) => {{
        let name = unsafe { CStr::from_ptr($cstr) };
        name.to_str().expect("invalid utf-8 in C string").to_string()
    }};
}

const PRIMITIVE_TYPES: &[(&str, &str)] = &[
    ("System.Void", "void"),
    ("System.Boolean", "bool"),
    ("System.Char", "char"),
    ("System.SByte", "sbyte"),
    ("System.Byte", "byte"),
    ("System.Int16", "short"),
    ("System.UInt16", "ushort"),
    ("System.Int32", "int"),
    ("System.UInt32", "uint"),
    ("System.Int64", "long"),
    ("System.UInt64", "ulong"),
    ("System.Single", "float"),
    ("System.Double", "double"),
    ("System.String", "string"),
    ("System.IntPtr", "IntPtr"),
    ("System.UIntPtr", "UIntPtr"),
    ("System.Object", "object"),
    ("&", ""),
];

#[derive(Debug, Error)]
pub enum Il2CppError {
    #[error(transparent)]
    Module(#[from] ModuleError),

    #[error("file not found {0}")]
    FileNotFound(&'static str),
    #[error("root path not found")]
    RootNotFound,
}

pub struct Il2CppApi {
    pub game_assembly: Module,
    pub unity_player: Module,
    pub functions: Il2CppFunctions,
    pub rva_log: String,
}

impl Il2CppApi {
    pub fn new(path: PathBuf) -> Result<Self, Il2CppError> {
        let game_assembly_path = path.join("GameAssembly.dll");
        let unity_player_path = path.join("UnityPlayer.dll");

        if !game_assembly_path.exists() {
            return Err(Il2CppError::FileNotFound("GameAssembly.dll"));
        }

        if !unity_player_path.exists() {
            return Err(Il2CppError::FileNotFound("UnityPlayer.dll"));
        }

        let game_assembly = Module::load(game_assembly_path).map_err(|err| {
            error!("failed to load GameAssembly.dll: {}", err);
            Il2CppError::from(err)
        })?;
        let unity_player = Module::load(unity_player_path).map_err(|err| {
            error!("failed to load UnityPlayer.dll: {}", err);
            Il2CppError::from(err)
        })?;

        debug!("GameAssembly.dll base=0x{:x} size=0x{:x}", game_assembly.handle as usize, game_assembly.size);
        debug!("UnityPlayer.dll base=0x{:x} size=0x{:x}", unity_player.handle as usize, unity_player.size);

        let (functions, rva_log) = Il2CppFunctions::new(unity_player.handle as usize, &[("GameAssembly", &game_assembly), ("UnityPlayer", &unity_player)]);
        info!("il2cpp api initialized");

        Ok(Il2CppApi {
            game_assembly,
            unity_player,
            functions,
            rva_log,
        })
    }

    pub fn domain_get(&self) -> *const Il2CppDomain {
        let function = get_function!(self, il2cpp_domain_get);
        let domain = function();
        assert!(!domain.is_null(), "il2cpp_domain_get returned null");
        domain
    }

    pub fn domain_get_assemblies(&self, domain: *const Il2CppDomain, size: *const usize) -> *const *const Il2CppAssembly {
        let function = get_function!(self, il2cpp_domain_get_assemblies);
        let assemblies = function(domain, size);
        assert!(!assemblies.is_null(), "il2cpp_domain_get_assemblies returned null");
        assemblies
    }

    pub fn assembly_get_image(&self, assembly: *const Il2CppAssembly) -> *const Il2CppImage {
        let function = get_function!(self, il2cpp_assembly_get_image);
        let image = function(assembly);
        assert!(!image.is_null(), "il2cpp_assembly_get_image returned null");
        image
    }

    pub fn image_get_name(&self, image: *const Il2CppImage) -> String {
        let function = get_function!(self, il2cpp_image_get_name);
        let name_c = function(image);
        assert!(!name_c.is_null(), "il2cpp_image_get_name returned null");
        cstr_to_string!(name_c)
    }

    pub fn image_get_class_count(&self, image: *const Il2CppImage) -> usize {
        let function = get_function!(self, il2cpp_image_get_class_count);
        function(image)
    }

    pub fn image_get_class(&self, image: *const Il2CppImage, index: usize) -> *const Il2CppClass {
        let function = get_function!(self, il2cpp_image_get_class);
        let class = function(image, index);
        assert!(!class.is_null(), "il2cpp_image_get_class returned null");
        class
    }

    pub fn class_get_fields(&self, class: *const Il2CppClass, iter: *const *const c_void) -> Option<*const FieldInfo> {
        let function = get_function!(self, il2cpp_class_get_fields);
        let result = function(class, iter);
        if result.is_null() {
            None
        } else {
            Some(result)
        }
    }

    pub fn class_get_interfaces(&self, class: *const Il2CppClass, iter: *const *const c_void) -> Option<*const Il2CppClass> {
        let function = get_function!(self, il2cpp_class_get_interfaces);
        let result = function(class, iter);
        if result.is_null() {
            None
        } else {
            Some(result)
        }
    }

    pub fn class_get_methods(&self, class: *const Il2CppClass, iter: *const *const c_void) -> Option<*const MethodInfo> {
        let function = get_function!(self, il2cpp_class_get_methods);
        let result = function(class, iter);
        if result.is_null() {
            None
        } else {
            Some(result)
        }
    }

    pub fn class_get_name(&self, class: *const Il2CppClass) -> String {
        let function = get_function!(self, il2cpp_class_get_name);
        let name_c = function(class);
        assert!(!name_c.is_null(), "il2cpp_class_get_name returned null");
        cstr_to_string!(name_c)
    }

    pub fn class_get_namespace(&self, class: *const Il2CppClass) -> String {
        let function = get_function!(self, il2cpp_class_get_namespace);
        let name_c = function(class);
        assert!(!name_c.is_null(), "il2cpp_class_get_namespace returned null");
        cstr_to_string!(name_c)
    }

    pub fn class_get_parent(&self, class: *const Il2CppClass) -> Option<*const Il2CppClass> {
        let function = get_function!(self, il2cpp_class_get_parent);
        let parent = function(class);
        if parent.is_null() {
            None
        } else {
            Some(parent)
        }
    }

    pub fn class_instance_size(&self, class: *const Il2CppClass) -> u32 {
        let function = get_function!(self, il2cpp_class_instance_size);
        function(class)
    }

    pub fn class_get_flags(&self, class: *const Il2CppClass) -> i32 {
        let function = get_function!(self, il2cpp_class_get_flags);
        function(class)
    }

    pub fn class_from_type(&self, class_type: *const Il2CppType) -> *const Il2CppClass {
        let function = get_function!(self, il2cpp_class_from_type);
        let class = function(class_type);
        assert!(!class.is_null(), "il2cpp_class_from_type returned null");
        class
    }

    pub fn class_is_enum(&self, class: *const Il2CppClass) -> bool {
        let function = get_function!(self, il2cpp_class_is_enum);
        function(class)
    }

    pub fn class_is_valuetype(&self, class: *const Il2CppClass) -> bool {
        let function = get_function!(self, il2cpp_class_is_valuetype);
        function(class)
    }

    pub fn field_get_flags(&self, field: *const FieldInfo) -> i32 {
        let function = get_function!(self, il2cpp_field_get_flags);
        function(field)
    }

    pub fn field_get_name(&self, field: *const FieldInfo) -> String {
        let function = get_function!(self, il2cpp_field_get_name);
        let name_c = function(field);
        assert!(!name_c.is_null(), "il2cpp_field_get_name returned null");
        cstr_to_string!(name_c)
    }

    pub fn field_get_offset(&self, field: *const FieldInfo) -> usize {
        let function = get_function!(self, il2cpp_field_get_offset);
        function(field)
    }

    pub fn field_get_type(&self, field: *const FieldInfo) -> *const Il2CppType {
        let function = get_function!(self, il2cpp_field_get_type);
        let field_type = function(field);
        assert!(!field_type.is_null(), "il2cpp_field_get_type returned null");
        field_type
    }

    pub fn method_get_return_type(&self, method: *const MethodInfo) -> *const Il2CppType {
        let function = get_function!(self, il2cpp_method_get_return_type);
        let return_type = function(method);
        assert!(!return_type.is_null(), "il2cpp_method_get_return_type returned null");
        return_type
    }

    pub fn method_get_name(&self, method: *const MethodInfo) -> String {
        let function = get_function!(self, il2cpp_method_get_name);
        let name_c = function(method);
        assert!(!name_c.is_null(), "il2cpp_method_get_name returned null");
        cstr_to_string!(name_c)
    }

    pub fn method_get_param_count(&self, method: *const MethodInfo) -> u32 {
        let function = get_function!(self, il2cpp_method_get_param_count);
        function(method)
    }

    pub fn method_get_param(&self, method: *const MethodInfo, index: u32) -> *const Il2CppType {
        let function = get_function!(self, il2cpp_method_get_param);
        let param = function(method, index);
        assert!(!param.is_null(), "il2cpp_method_get_param returned null");
        param
    }

    pub fn type_get_name(&self, _type: *const Il2CppType) -> String {
        let function = get_function!(self, il2cpp_type_get_name);
        let name_c = function(_type);
        assert!(!name_c.is_null(), "il2cpp_type_get_name returned null");

        let mut name = cstr_to_string!(name_c);

        for &(k, v) in PRIMITIVE_TYPES {
            name = name.replace(k, v);
        }

        name
    }

    pub fn type_is_byref(&self, type_: *const Il2CppType) -> bool {
        let function = get_function!(self, il2cpp_type_is_byref);
        function(type_)
    }

    pub fn type_get_attrs(&self, type_: *const Il2CppType) -> u32 {
        let function = get_function!(self, il2cpp_type_get_attrs);
        function(type_)
    }

    pub fn type_get_type(&self, type_: *const Il2CppType) -> u32 {
        let function = get_function!(self, il2cpp_type_get_type);
        function(type_)
    }

    pub fn type_get_class_or_element_class(&self, type_: *const Il2CppType) -> *const Il2CppClass {
        let function = get_function!(self, il2cpp_type_get_class_or_element_class);
        let class = function(type_);
        assert!(!class.is_null(), "il2cpp_type_get_class_or_element_class returned null");
        class
    }

    pub fn class_get_type(&self, class: *const Il2CppClass) -> *const Il2CppType {
        let function = get_function!(self, il2cpp_class_get_type);
        let type_ = function(class);
        assert!(!type_.is_null(), "il2cpp_class_get_type returned null");
        type_
    }

    pub fn class_get_image(&self, class: *const Il2CppClass) -> *const Il2CppImage {
        let function = get_function!(self, il2cpp_class_get_image);
        let image = function(class);
        assert!(!image.is_null(), "il2cpp_class_get_image returned null");
        image
    }

    pub fn class_get_declaring_type(&self, class: *const Il2CppClass) -> Option<*const Il2CppClass> {
        let function = get_function!(self, il2cpp_class_get_declaring_type);
        let parent = function(class);
        if parent.is_null() {
            None
        } else {
            Some(parent)
        }
    }

    pub fn class_is_generic(&self, class: *const Il2CppClass) -> bool {
        let function = get_function!(self, il2cpp_class_is_generic);
        function(class)
    }

    pub fn class_is_inflated(&self, class: *const Il2CppClass) -> bool {
        let function = get_function!(self, il2cpp_class_is_inflated);
        function(class)
    }

    pub fn class_num_fields(&self, class: *const Il2CppClass) -> usize {
        let function = get_function!(self, il2cpp_class_num_fields);
        function(class)
    }

    pub fn method_get_param_name(&self, method: *const MethodInfo, index: u32) -> String {
        let function = get_function!(self, il2cpp_method_get_param_name);
        let name_c = function(method, index);
        assert!(!name_c.is_null(), "il2cpp_method_get_param_name returned null");
        cstr_to_string!(name_c)
    }

    pub fn method_get_flags(&self, method: *const MethodInfo, impl_flags: *mut u32) -> u32 {
        let function = get_function!(self, il2cpp_method_get_flags);
        function(method, impl_flags)
    }

    pub fn method_get_token(&self, method: *const MethodInfo) -> u32 {
        let function = get_function!(self, il2cpp_method_get_token);
        function(method)
    }

    pub fn method_is_generic(&self, method: *const MethodInfo) -> bool {
        let function = get_function!(self, il2cpp_method_is_generic);
        function(method)
    }

    pub fn method_is_inflated(&self, method: *const MethodInfo) -> bool {
        let function = get_function!(self, il2cpp_method_is_inflated);
        function(method)
    }

    pub fn method_is_instance(&self, method: *const MethodInfo) -> bool {
        let function = get_function!(self, il2cpp_method_is_instance);
        function(method)
    }

    pub fn class_get_properties(&self, class: *const Il2CppClass, iter: *const *const c_void) -> Option<*const PropertyInfo> {
        let function = get_function!(self, il2cpp_class_get_properties);
        let result = function(class, iter);
        if result.is_null() {
            None
        } else {
            Some(result)
        }
    }

    pub fn property_get_name(&self, prop: *const PropertyInfo) -> String {
        let function = get_function!(self, il2cpp_property_get_name);
        let name_c = function(prop);
        assert!(!name_c.is_null(), "il2cpp_property_get_name returned null");
        cstr_to_string!(name_c)
    }

    pub fn property_get_get_method(&self, prop: *const PropertyInfo) -> Option<*const MethodInfo> {
        let function = get_function!(self, il2cpp_property_get_get_method);
        let method = function(prop);
        if method.is_null() {
            None
        } else {
            Some(method)
        }
    }

    pub fn property_get_set_method(&self, prop: *const PropertyInfo) -> Option<*const MethodInfo> {
        let function = get_function!(self, il2cpp_property_get_set_method);
        let method = function(prop);
        if method.is_null() {
            None
        } else {
            Some(method)
        }
    }

    pub fn property_get_flags(&self, prop: *const PropertyInfo) -> u32 {
        let function = get_function!(self, il2cpp_property_get_flags);
        function(prop)
    }

    pub fn property_get_parent(&self, prop: *const PropertyInfo) -> *const Il2CppClass {
        let function = get_function!(self, il2cpp_property_get_parent);
        let class = function(prop);
        assert!(!class.is_null(), "il2cpp_property_get_parent returned null");
        class
    }

    pub fn class_get_events(&self, class: *const Il2CppClass, iter: *const *const c_void) -> Option<*const EventInfo> {
        let function = get_function!(self, il2cpp_class_get_events);
        let result = function(class, iter);
        if result.is_null() {
            None
        } else {
            Some(result)
        }
    }

    pub fn event_get_name(&self, event: *const EventInfo) -> String {
        let name_c = unsafe { (*event).name };
        assert!(!name_c.is_null(), "EventInfo.name is null");
        cstr_to_string!(name_c)
    }

    pub fn event_get_add_method(&self, event: *const EventInfo) -> Option<*const MethodInfo> {
        let method = unsafe { (*event).add };
        if method.is_null() {
            None
        } else {
            Some(method)
        }
    }

    pub fn event_get_remove_method(&self, event: *const EventInfo) -> Option<*const MethodInfo> {
        let method = unsafe { (*event).remove };
        if method.is_null() {
            None
        } else {
            Some(method)
        }
    }

    pub fn event_get_raise_method(&self, event: *const EventInfo) -> Option<*const MethodInfo> {
        let method = unsafe { (*event).raise };
        if method.is_null() {
            None
        } else {
            Some(method)
        }
    }

    pub fn field_get_value(&self, obj: *const c_void, field: *const FieldInfo, value: *mut c_void) {
        let function = get_function!(self, il2cpp_field_get_value);
        function(obj, field, value)
    }

    pub fn field_static_get_value(&self, field: *const FieldInfo, value: *mut c_void) {
        let function = get_function!(self, il2cpp_field_static_get_value);
        function(field, value)
    }
}

static API: OnceLock<Il2CppApi> = OnceLock::new();

pub fn get_il2cpp_api() -> Result<&'static Il2CppApi, Box<dyn Error>> {
    if let Some(api) = API.get() {
        return Ok(api);
    }

    let exe_path = std::env::current_exe()?;
    let root_path = exe_path.parent().ok_or(Il2CppError::RootNotFound)?.to_path_buf();
    let api = Il2CppApi::new(root_path)?;

    if API.set(api).is_err() {
        return Ok(API.get().ok_or("Failed to get the il2cpp api")?);
    }

    Ok(API.get().ok_or("Failed to get the il2cpp api")?)
}

use std::{
    collections::{HashMap, HashSet},
    ffi::c_void,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    ptr::null,
};

use log::{debug, info, warn};
use serde_json::{json, Value};

use crate::{
    api,
    api::Il2CppApi,
    constants::*,
    types::{FieldInfo, Il2CppClass},
};

// ── dump.cs ─────────────────────────────────────────────────────────────────

fn write_images() -> String {
    let il2cpp = api::get_il2cpp_api().unwrap();
    let mut output = String::new();

    let domain = il2cpp.domain_get();

    let assembly_size: usize = 0;
    let assemblies = il2cpp.domain_get_assemblies(domain, &assembly_size);

    for i in 0..assembly_size {
        let assembly = unsafe { *assemblies.offset(i as isize) };

        if assembly.is_null() {
            continue;
        }

        let image = il2cpp.assembly_get_image(assembly);
        let name = il2cpp.image_get_name(image);

        let fmt = format!("// Image {}: {}\n", i, name);
        output.push_str(fmt.as_str());
    }

    output
}

fn write_fields(class: *const Il2CppClass, is_valuetype: bool) -> String {
    let il2cpp = api::get_il2cpp_api().unwrap();
    let mut output = String::new();

    output.push_str("\n\t// Fields\n");

    let field_iter: *const c_void = null();

    while let Some(field) = il2cpp.class_get_fields(class, &field_iter) {
        output.push_str("\t");

        let flags = il2cpp.field_get_flags(field);
        let access = flags & FIELD_ATTRIBUTE_FIELD_ACCESS_MASK;

        let access_str = match access {
            FIELD_ATTRIBUTE_PRIVATE => "private ",
            FIELD_ATTRIBUTE_PUBLIC => "public ",
            FIELD_ATTRIBUTE_FAMILY => "protected ",
            FIELD_ATTRIBUTE_ASSEMBLY | FIELD_ATTRIBUTE_FAM_AND_ASSEM => "internal ",
            FIELD_ATTRIBUTE_FAM_OR_ASSEM => "protected internal ",
            _ => "",
        };

        output.push_str(access_str);

        let mut is_static = false;

        if flags & FIELD_ATTRIBUTE_LITERAL != 0 {
            output.push_str("const ");
        } else {
            if flags & FIELD_ATTRIBUTE_STATIC != 0 {
                is_static = true;
                output.push_str("static ");
            }

            if flags & FIELD_ATTRIBUTE_INIT_ONLY != 0 {
                output.push_str("readonly ");
            }
        }

        let field_type = il2cpp.field_get_type(field);
        let mut field_offset = il2cpp.field_get_offset(field);

        if is_valuetype && !is_static && field_offset > 0 {
            field_offset -= 0x10;
        }

        let type_name = il2cpp.type_get_name(field_type);
        let field_name = il2cpp.field_get_name(field);

        let fmt = format!("{} {}; // 0x{:x}\n", type_name, field_name, field_offset);
        output.push_str(fmt.as_str());
    }

    output
}

fn write_methods(class: *const Il2CppClass) -> String {
    let il2cpp = api::get_il2cpp_api().unwrap();
    let mut output = String::new();

    output.push_str("\n\t// Methods\n");

    let method_iter: *const c_void = null();

    while let Some(method) = il2cpp.class_get_methods(class, &method_iter) {
        output.push_str("\n");

        let pointer = unsafe { (*method).method_pointer as usize };

        if pointer != 0 {
            let offset = pointer - il2cpp.game_assembly.handle as usize;
            let fmt = format!("\t// RVA: 0x{:x} VA: 0x{:x}\n\t", offset, offset + 0x180000000);
            output.push_str(fmt.as_str());
        } else {
            output.push_str("\t// RVA: 0x0 VA: 0x0\n\t");
        }

        let flags = unsafe { (*method).flags } as i32;
        let access = flags & METHOD_ATTRIBUTE_MEMBER_ACCESS_MASK;

        let access_str = match access {
            METHOD_ATTRIBUTE_PRIVATE => "private ",
            METHOD_ATTRIBUTE_PUBLIC => "public ",
            METHOD_ATTRIBUTE_FAMILY => "protected ",
            METHOD_ATTRIBUTE_ASSEM | METHOD_ATTRIBUTE_FAM_AND_ASSEM => "internal ",
            METHOD_ATTRIBUTE_FAM_OR_ASSEM => "protected internal ",
            _ => "",
        };

        output.push_str(access_str);

        if flags & METHOD_ATTRIBUTE_STATIC != 0 {
            output.push_str("static ");
        }

        if flags & METHOD_ATTRIBUTE_ABSTRACT != 0 {
            output.push_str("abstract ");

            if flags & METHOD_ATTRIBUTE_VTABLE_LAYOUT_MASK == METHOD_ATTRIBUTE_REUSE_SLOT {
                output.push_str("override ");
            }
        } else if flags & METHOD_ATTRIBUTE_FINAL != 0 && flags & METHOD_ATTRIBUTE_VTABLE_LAYOUT_MASK == METHOD_ATTRIBUTE_REUSE_SLOT {
            output.push_str("sealed override ");
        } else if flags & METHOD_ATTRIBUTE_VIRTUAL != 0 {
            if flags & METHOD_ATTRIBUTE_VTABLE_LAYOUT_MASK == METHOD_ATTRIBUTE_NEW_SLOT {
                output.push_str("virtual ");
            } else {
                output.push_str("override ");
            }
        }

        if flags & METHOD_ATTRIBUTE_PINVOKE_IMPL != 0 {
            output.push_str("extern ");
        }

        let return_type = il2cpp.method_get_return_type(method);

        if il2cpp.type_is_byref(return_type) {
            output.push_str("ref ");
        }

        let return_name = il2cpp.type_get_name(return_type);
        let method_name = il2cpp.method_get_name(method);

        let fmt = format!("{} {}(", return_name, method_name);
        output.push_str(fmt.as_str());

        let param_count = il2cpp.method_get_param_count(method);

        for i in 0..param_count {
            let param = il2cpp.method_get_param(method, i);
            let attrs = il2cpp.type_get_attrs(param) as i32;

            if il2cpp.type_is_byref(param) {
                if attrs & PARAM_ATTRIBUTE_OUT != 0 && attrs & PARAM_ATTRIBUTE_IN == 0 {
                    output.push_str("out ");
                } else if attrs & PARAM_ATTRIBUTE_IN != 0 && attrs & PARAM_ATTRIBUTE_OUT == 0 {
                    output.push_str("in ");
                } else {
                    output.push_str("ref ");
                }
            } else {
                if attrs & PARAM_ATTRIBUTE_IN != 0 {
                    output.push_str("[In] ");
                }

                if attrs & PARAM_ATTRIBUTE_OUT != 0 {
                    output.push_str("[Out] ");
                }
            }

            let type_name = il2cpp.type_get_name(param);
            let param_name = if il2cpp.functions.il2cpp_method_get_param_name.is_some() {
                il2cpp.method_get_param_name(method, i)
            } else {
                format!("param_{}", i)
            };
            let fmt = format!("{} {}{}", type_name, param_name, if i != param_count - 1 { ", " } else { "" });
            output.push_str(fmt.as_str());
        }

        output.push_str(") { }\n");
    }

    output
}

fn write_properties(class: *const Il2CppClass) -> String {
    let il2cpp = api::get_il2cpp_api().unwrap();

    if il2cpp.functions.il2cpp_class_get_properties.is_none() {
        return String::new();
    }

    let mut output = String::new();
    let mut has_header = false;
    let prop_iter: *const c_void = null();

    while let Some(prop) = il2cpp.class_get_properties(class, &prop_iter) {
        if !has_header {
            output.push_str("\n\t// Properties\n");
            has_header = true;
        }

        let name = il2cpp.property_get_name(prop);

        let type_name = if let Some(getter) = il2cpp.property_get_get_method(prop) {
            il2cpp.type_get_name(il2cpp.method_get_return_type(getter))
        } else if let Some(setter) = il2cpp.property_get_set_method(prop) {
            let param_count = il2cpp.method_get_param_count(setter);
            if param_count > 0 {
                il2cpp.type_get_name(il2cpp.method_get_param(setter, param_count - 1))
            } else {
                "???".to_string()
            }
        } else {
            "???".to_string()
        };

        let has_get = il2cpp.property_get_get_method(prop).is_some();
        let has_set = il2cpp.property_get_set_method(prop).is_some();

        let accessors = match (has_get, has_set) {
            (true, true) => "{ get; set; }",
            (true, false) => "{ get; }",
            (false, true) => "{ set; }",
            (false, false) => "{ }",
        };

        let fmt = format!("\t{} {} {}\n", type_name, name, accessors);
        output.push_str(fmt.as_str());
    }

    output
}

fn write_events(class: *const Il2CppClass) -> String {
    let il2cpp = api::get_il2cpp_api().unwrap();

    if il2cpp.functions.il2cpp_class_get_events.is_none() {
        return String::new();
    }

    let mut output = String::new();
    let mut has_header = false;
    let event_iter: *const c_void = null();

    while let Some(event) = il2cpp.class_get_events(class, &event_iter) {
        if !has_header {
            output.push_str("\n\t// Events\n");
            has_header = true;
        }

        let name = il2cpp.event_get_name(event);

        let type_name = if let Some(add) = il2cpp.event_get_add_method(event) {
            let param_count = il2cpp.method_get_param_count(add);
            if param_count > 0 {
                il2cpp.type_get_name(il2cpp.method_get_param(add, 0))
            } else {
                "???".to_string()
            }
        } else {
            "???".to_string()
        };

        output.push_str(format!("\tevent {} {};\n", type_name, name).as_str());
    }

    output
}

fn write_class(class: *const Il2CppClass) -> String {
    let il2cpp = api::get_il2cpp_api().unwrap();
    let mut output = String::new();

    let namespace = il2cpp.class_get_namespace(class);
    let fmt = format!("\n// Namespace: {}\n", namespace);
    output.push_str(fmt.as_str());

    let flags = il2cpp.class_get_flags(class);

    if flags & TYPE_ATTRIBUTE_SERIALIZABLE != 0 {
        output.push_str("[Serializable]\n");
    }

    let visibility = flags & TYPE_ATTRIBUTE_VISIBILITY_MASK;

    let visibility_str = match visibility {
        TYPE_ATTRIBUTE_PUBLIC | TYPE_ATTRIBUTE_NESTED_PUBLIC => "public ",
        TYPE_ATTRIBUTE_NOT_PUBLIC | TYPE_ATTRIBUTE_NESTED_FAM_AND_ASSEM | TYPE_ATTRIBUTE_NESTED_ASSEMBLY => "internal ",
        TYPE_ATTRIBUTE_NESTED_PRIVATE => "private ",
        TYPE_ATTRIBUTE_NESTED_FAMILY => "protected ",
        TYPE_ATTRIBUTE_NESTED_FAM_OR_ASSEM => "protected internal ",
        _ => "",
    };

    output.push_str(visibility_str);

    let is_valuetype = il2cpp.class_is_valuetype(class);
    let is_enum = il2cpp.class_is_enum(class);

    if flags & TYPE_ATTRIBUTE_ABSTRACT != 0 && flags & TYPE_ATTRIBUTE_SEALED != 0 {
        output.push_str("static ");
    } else if !(flags & TYPE_ATTRIBUTE_INTERFACE != 0) && flags & TYPE_ATTRIBUTE_ABSTRACT != 0 {
        output.push_str("abstract ");
    } else if !is_valuetype && !is_enum && flags & TYPE_ATTRIBUTE_SEALED != 0 {
        output.push_str("sealed ");
    }

    if flags & TYPE_ATTRIBUTE_INTERFACE != 0 {
        output.push_str("interface ");
    } else if is_enum {
        output.push_str("enum ");
    } else if is_valuetype {
        output.push_str("struct ");
    } else {
        output.push_str("class ");
    }

    let name = il2cpp.class_get_name(class);
    output.push_str(name.as_str());

    let mut extends = vec![];

    if let Some(parent) = il2cpp.class_get_parent(class) {
        let name = il2cpp.class_get_name(parent);

        if !is_valuetype && !is_enum && name != "Object" {
            extends.push(name);
        }
    }

    let interface_ier: *const c_void = null();

    while let Some(interface) = il2cpp.class_get_interfaces(class, &interface_ier) {
        let name = il2cpp.class_get_name(interface);
        extends.push(name);
    }

    if !extends.is_empty() {
        let fmt = format!(" : {}", extends.join(", "));
        output.push_str(fmt.as_str());
    }

    output.push_str("\n{");
    output.push_str(write_fields(class, is_valuetype).as_str());
    output.push_str(write_properties(class).as_str());
    output.push_str(write_events(class).as_str());
    output.push_str(write_methods(class).as_str());
    output.push_str("}\n");

    output
}

pub fn dump_cs(out_dir: &Path) {
    info!("dumping dump.cs");
    let il2cpp = api::get_il2cpp_api().unwrap();
    let mut output = String::new();

    // images header
    let domain = il2cpp.domain_get();
    let assembly_size: usize = 0;
    let assemblies = il2cpp.domain_get_assemblies(domain, &assembly_size);

    output.push_str(&write_images());

    for i in 0..assembly_size {
        let assembly = unsafe { *assemblies.offset(i as isize) };
        if assembly.is_null() {
            continue;
        }
        let image = il2cpp.assembly_get_image(assembly);
        let class_count = il2cpp.image_get_class_count(image);

        for j in 0..class_count {
            let class = il2cpp.image_get_class(image, j);
            if class.is_null() {
                continue;
            }
            output.push_str(write_class(class).as_str());
        }
    }

    let out_path = out_dir.join("dump.cs");
    let mut file = File::create(&out_path).unwrap();
    file.write_all(output.as_bytes()).unwrap();
    info!("written {}", out_path.display());
}

// ── il2cpp.json ─────────────────────────────────────────────────────────────

#[inline(always)]
fn compute_pair_hash(index: u32) -> u32 {
    let v = (PAIR_HASH_MUL1.wrapping_mul(index as u64)) >> 0x16;
    let v = (PAIR_HASH_MUL2 as u64).wrapping_mul(v) >> 0x10;
    PAIR_HASH_MUL3.wrapping_mul(v as u32)
}

fn compute_field_hash(source_index: u32) -> u32 {
    let v = (FIELD_HASH_MUL.wrapping_mul(source_index as u64)) ^ FIELD_HASH_XOR;
    let v = FIELD_HASH_MUL2.wrapping_mul(v).wrapping_add(FIELD_HASH_ADD) ^ FIELD_HASH_XOR2;
    ((FIELD_HASH_MUL3.wrapping_mul(v)) >> 0xB) as u32
}

fn decode_list_entry(lists_base: usize, index: u32) -> (u32, u32) {
    let val_a = unsafe { *((lists_base + index as usize * 4) as *const u32) };
    let val_b = unsafe { *((lists_base + (index + 1) as usize * 4) as *const u32) };

    let hash_a = ((LISTS_MUL.wrapping_mul(index as u64)) >> 0xA) as u32;
    let hash_b = ((LISTS_MUL.wrapping_mul((index + 1) as u64)) >> 0xA) as u32;

    let dec_a = (val_a ^ LISTS_XOR_A).wrapping_add(hash_a ^ LISTS_XOR_B);
    let dec_b = (val_b ^ LISTS_XOR_A).wrapping_add(hash_b ^ LISTS_XOR_B);

    let start = dec_a.wrapping_add(LISTS_ADD);
    let count = dec_b.wrapping_add(LISTS_ADD).wrapping_sub(start);

    (start, count)
}

struct DecodedPair {
    usage_type: u32,
    source_index: u32,
    dest_index: u32,
}

#[inline(always)]
fn decode_pair(pairs_base: usize, index: u32) -> DecodedPair {
    let off = index as usize * 8;
    let hash = compute_pair_hash(index);

    let raw_encoded = unsafe { *((pairs_base + off + 4) as *const u32) };
    let raw_dest = unsafe { *((pairs_base + off) as *const u32) };

    let decoded_encoded = raw_encoded ^ hash;
    let usage_type = decoded_encoded >> 0x1D;
    let source_index = (decoded_encoded ^ PAIR_XOR_ENCODED) & 0x1FFFFFFF;
    let dest_index = raw_dest ^ hash ^ PAIR_XOR_DEST;

    DecodedPair { usage_type, source_index, dest_index }
}

struct StringDecryptCtx {
    offsets_base: usize,
    data_base: isize,
}

impl StringDecryptCtx {
    fn new(metadata: usize, header: usize) -> Self {
        let offsets_offset = unsafe { (*((header + 0xE0) as *const u32)).wrapping_add(STRING_LITERAL_OFFSETS_KEY) } as i32;
        let offsets_base = (metadata as isize + offsets_offset as isize) as usize;
        let data_base_offset = unsafe { (*((header + 0x150) as *const i32)) as i64 } ^ STRING_LITERAL_DATA_KEY as i64;
        let data_base = metadata as isize + data_base_offset as isize;
        Self { offsets_base, data_base }
    }

    fn decrypt(&self, source_index: u32) -> Option<String> {
        unsafe {
            let idx = source_index as usize;
            let raw_offset = *((self.offsets_base + idx * 4) as *const u32);
            let raw_offset_next = *((self.offsets_base + idx * 4 + 4) as *const u32);

            let hash = ((STR_DECRYPT_MUL.wrapping_mul(source_index as u64)) >> 0xB) as u32;
            let dec_hash = STR_DECRYPT_HASH_MUL.wrapping_mul(hash);
            let str_data_offset = raw_offset.wrapping_add(dec_hash).wrapping_add(STR_DECRYPT_ADD) as i32;

            let hash_next = ((STR_DECRYPT_MUL.wrapping_mul(source_index as u64).wrapping_add(STR_DECRYPT_MUL)) >> 0xB) as u32;
            let dec_hash_next = STR_DECRYPT_HASH_MUL.wrapping_mul(hash_next);

            let str_len = (dec_hash_next as i32).wrapping_sub(str_data_offset).wrapping_add(raw_offset_next as i32).wrapping_add(STR_DECRYPT_ADD as i32) as u32;

            if str_len == 0 || str_len > 1_000_000 {
                return None;
            }

            let data_ptr = (self.data_base + str_data_offset as isize) as usize;
            let str_len = str_len as usize;

            let mut key = STR_DECRYPT_KEY_MUL1.wrapping_mul(STR_DECRYPT_KEY_MUL2.wrapping_mul(source_index as u64) ^ STR_DECRYPT_KEY_XOR);

            let blocks = (str_len + 7) / 8;
            let mut buf = vec![0u8; blocks * 8];
            let dst = buf.as_mut_ptr() as *mut u64;

            for i in 0..blocks {
                let enc = *((data_ptr + i * 8) as *const u64);
                *dst.add(i) = enc ^ key;
                key = key.wrapping_add(STR_DECRYPT_KEY_INC);
            }

            buf.truncate(str_len);
            String::from_utf8(buf).ok()
        }
    }
}

struct FieldDefaultCtx {
    defaults: HashMap<u32, (usize, usize)>,
}

impl FieldDefaultCtx {
    fn new(metadata: usize, header: usize) -> Self {
        let fdv_offset = unsafe { (*((header + 0x84) as *const u32)) ^ FIELD_DEFAULT_VALUES_OFFSET_XOR } as i32;
        let fdv_size = unsafe { (*((header + 0xAC) as *const u32)).wrapping_sub(FIELD_DEFAULT_VALUES_SIZE_SUB) };
        let data_offset = unsafe { (*((header + 0x54) as *const u32)) ^ FIELD_DEFAULT_DATA_OFFSET_XOR } as i32;

        let fdv_base = (metadata as isize + fdv_offset as isize) as usize;
        let data_base = (metadata as isize + data_offset as isize) as usize;
        let count = fdv_size as usize / 12;

        let mut defaults = HashMap::new();
        for i in 0..count {
            let entry = fdv_base + i * 12;
            let data_index = unsafe { *((entry) as *const u32) };
            let field_index = unsafe { *((entry + 4) as *const u32) };
            if data_index != 0xFFFFFFFF && field_index != 0xFFFFFFFF {
                let data_ptr = data_base + data_index as usize;
                let next_data_index = if i + 1 < count {
                    let next_di = unsafe { *((entry + 12) as *const u32) };
                    if next_di != 0xFFFFFFFF {
                        next_di as usize
                    } else {
                        data_index as usize + 256
                    }
                } else {
                    data_index as usize + 256
                };
                let next_data_ptr = data_base + next_data_index;
                defaults.insert(field_index, (data_ptr, next_data_ptr));
            }
        }
        Self { defaults }
    }

    fn get_data(&self, field_index: u32, size: usize) -> Option<&[u8]> {
        let &(data_ptr, _) = self.defaults.get(&field_index)?;
        if size == 0 || size > 0x10000 {
            return None;
        }
        Some(unsafe { std::slice::from_raw_parts(data_ptr as *const u8, size) })
    }
}

struct FieldRefCtx {
    refs_base: usize,
    type_cache: HashMap<i32, *const Il2CppClass>,
}

impl FieldRefCtx {
    fn new(metadata: usize, header: usize) -> Self {
        let refs_offset = unsafe { (*((header + 0x18) as *const u32)).wrapping_add(FIELD_REFS_OFFSET_KEY) } as i32;
        let refs_base = (metadata as isize + refs_offset as isize) as usize;
        Self { refs_base, type_cache: HashMap::new() }
    }

    fn get_type_info_cached(&mut self, type_index: i32, get_type_info: unsafe extern "C" fn(i32, bool) -> *const Il2CppClass) -> *const Il2CppClass {
        *self.type_cache.entry(type_index).or_insert_with(|| unsafe { get_type_info(type_index, true) })
    }

    fn resolve(&mut self, source_index: u32, get_type_info: unsafe extern "C" fn(i32, bool) -> *const Il2CppClass) -> Option<(*const Il2CppClass, *const FieldInfo, u32)> {
        unsafe {
            let hash = compute_field_hash(source_index);
            let raw_type = *((self.refs_base + source_index as usize * 8) as *const u32);
            let raw_field = *((self.refs_base + source_index as usize * 8 + 4) as *const u32);

            let type_index = raw_type.wrapping_sub(hash).wrapping_add(FIELD_TYPE_INDEX_ADD) as i32;
            let field_index = (raw_field ^ FIELD_INDEX_XOR).wrapping_sub(hash) as i32;

            let class = self.get_type_info_cached(type_index, get_type_info);
            if class.is_null() {
                return None;
            }

            let fields_ptr = *((class as usize + 0x10) as *const usize);
            if fields_ptr == 0 {
                return None;
            }

            let field = (fields_ptr + field_index as usize * 0x20) as *const FieldInfo;

            let type_def = *((class as usize + 0x40) as *const usize);
            let field_start = if type_def != 0 { *((type_def + 0x14) as *const u16) as u32 } else { 0 };
            let global_field_index = field_start + field_index as u32;

            Some((class, field, global_field_index))
        }
    }
}

unsafe fn scan_metadata_tokens(base: usize, size: usize) -> Vec<u32> {
    let target_addr = base + INIT_METADATA_RANGE_RVA;
    let mut tokens = Vec::new();
    let slice = std::slice::from_raw_parts(base as *const u8, size);

    let end = size.saturating_sub(10);
    let mut i = 0;
    while i < end {
        if slice[i] == 0xB9 && slice[i + 5] == 0xE8 {
            let rel32 = *(slice.as_ptr().add(i + 6) as *const i32);
            let call_site = base + i + 5;
            let call_target = (call_site as isize + 5 + rel32 as isize) as usize;

            if call_target == target_addr {
                let imm32 = *(slice.as_ptr().add(i + 1) as *const u32);
                tokens.push(imm32);
                i += 10;
                continue;
            }
        }
        i += 1;
    }

    tokens.sort_unstable();
    tokens.dedup();
    tokens
}

fn get_field_value_size(il2cpp: &Il2CppApi, field: *const FieldInfo) -> usize {
    let field_type = il2cpp.field_get_type(field);
    let type_enum = il2cpp.type_get_type(field_type);
    match type_enum {
        IL2CPP_TYPE_BOOLEAN | IL2CPP_TYPE_I1 | IL2CPP_TYPE_U1 => 1,
        IL2CPP_TYPE_CHAR | IL2CPP_TYPE_I2 | IL2CPP_TYPE_U2 => 2,
        IL2CPP_TYPE_I4 | IL2CPP_TYPE_U4 | IL2CPP_TYPE_R4 => 4,
        IL2CPP_TYPE_I8 | IL2CPP_TYPE_U8 | IL2CPP_TYPE_R8 => 8,
        IL2CPP_TYPE_I | IL2CPP_TYPE_U => 8,
        IL2CPP_TYPE_VALUETYPE => {
            let klass = il2cpp.class_from_type(field_type);
            let instance_size = il2cpp.class_instance_size(klass);
            if instance_size > 0x10 {
                (instance_size - 0x10) as usize
            } else {
                0
            }
        }
        _ => 8,
    }
}

fn read_field_hex(fdv_ctx: &FieldDefaultCtx, global_field_index: u32, size: usize) -> String {
    match fdv_ctx.get_data(global_field_index, size) {
        Some(buf) => buf.iter().map(|b| format!("{:02X}", b)).collect(),
        None => String::new(),
    }
}

pub fn dump_json(out_dir: &Path) {
    info!("dumping il2cpp.json");
    let il2cpp = api::get_il2cpp_api().unwrap();
    let ga_base = il2cpp.game_assembly.handle as usize;
    let ga_size = il2cpp.game_assembly.size;

    let header = unsafe { *((ga_base + HEADER_PTR_RVA) as *const usize) };
    let metadata = unsafe { *((ga_base + METADATA_PTR_RVA) as *const usize) };
    let dispatch = unsafe { *((ga_base + DISPATCH_PTR_RVA) as *const usize) };

    if header == 0 || metadata == 0 {
        warn!("metadata globals not initialized, skipping il2cpp.json");
        return;
    }

    let pairs_offset = unsafe { (*((header + 0xF8) as *const u32)).wrapping_add(PAIRS_OFFSET_KEY) } as i32;
    let pairs_base = (metadata as isize + pairs_offset as isize) as usize;

    let pairs_data_offset = unsafe { *((header + PAIRS_BASE_OFFSET as usize) as *const i32) } as i64 ^ PAIRS_BASE_XOR as i64;
    let pairs_data_base = (metadata as i64 + pairs_data_offset) as usize;

    let tokens = unsafe { scan_metadata_tokens(ga_base, ga_size) };
    debug!("found {} unique metadata tokens", tokens.len());

    let get_type_info: unsafe extern "C" fn(i32, bool) -> *const Il2CppClass = unsafe { std::mem::transmute(ga_base + GET_TYPE_INFO_RVA) };

    let str_ctx = StringDecryptCtx::new(metadata, header);
    let mut field_ctx = FieldRefCtx::new(metadata, header);
    let fdv_ctx = FieldDefaultCtx::new(metadata, header);

    let fi_array = if dispatch != 0 { unsafe { *((dispatch + 0x30) as *const usize) } } else { 0 };
    let sl_array = if dispatch != 0 { unsafe { *((dispatch + 0x28) as *const usize) } } else { 0 };

    let mut string_entries: Vec<Value> = Vec::new();
    let mut field_entries: Vec<Value> = Vec::new();

    let mut seen_addrs: HashSet<usize> = HashSet::new();

    for &token in &tokens {
        let (start, count) = decode_list_entry(pairs_base, token);

        for i in 0..count {
            let pair = decode_pair(pairs_data_base, start.wrapping_add(i));

            match pair.usage_type {
                4 if fi_array != 0 => {
                    let addr = fi_array + pair.dest_index as usize * 8 - ga_base;
                    if seen_addrs.insert(addr) {
                        if let Some((owner_class, field, global_field_index)) = field_ctx.resolve(pair.source_index, get_type_info) {
                            let field_name = il2cpp.field_get_name(field);
                            let class_name = il2cpp.class_get_name(owner_class);
                            let ns = il2cpp.class_get_namespace(owner_class);
                            let owner = if ns.is_empty() { class_name } else { format!("{}.{}", ns, class_name) };
                            let flags = il2cpp.field_get_flags(field);
                            let value = if (flags & FIELD_ATTRIBUTE_HAS_FIELD_RVA) != 0 {
                                let size = get_field_value_size(il2cpp, field);
                                read_field_hex(&fdv_ctx, global_field_index, size)
                            } else {
                                String::new()
                            };
                            field_entries.push(json!({
                                "Address": addr,
                                "Name": format!("{}_{}", owner, field_name),
                                "Value": value,
                            }));
                        }
                    }
                }
                5 if sl_array != 0 => {
                    let addr = sl_array + pair.dest_index as usize * 8 - ga_base;
                    if seen_addrs.insert(addr) {
                        if let Some(s) = str_ctx.decrypt(pair.source_index) {
                            string_entries.push(json!({
                                "Address": addr,
                                "Value": s,
                            }));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Collect methods from runtime
    debug!("decoded {} strings, {} fields", string_entries.len(), field_entries.len());
    let domain = il2cpp.domain_get();
    let assembly_count: usize = 0;
    let assemblies = il2cpp.domain_get_assemblies(domain, &assembly_count);

    let mut method_entries: Vec<Value> = Vec::new();
    let mut method_addrs: HashSet<usize> = HashSet::new();
    let mut duplicates: HashMap<String, u32> = HashMap::new();

    for i in 0..assembly_count {
        let assembly = unsafe { *assemblies.offset(i as isize) };
        if assembly.is_null() {
            continue;
        }
        let image = il2cpp.assembly_get_image(assembly);
        let class_count = il2cpp.image_get_class_count(image);

        for j in 0..class_count {
            let class = il2cpp.image_get_class(image, j);
            if class.is_null() {
                continue;
            }

            let class_name = il2cpp.class_get_name(class);
            let namespace = il2cpp.class_get_namespace(class);
            let type_prefix = if namespace.is_empty() { class_name.clone() } else { format!("{}.{}", namespace, class_name) };

            let method_iter: *const c_void = null();
            while let Some(method) = il2cpp.class_get_methods(class, &method_iter) {
                let pointer = unsafe { (*method).method_pointer as usize };
                if pointer <= ga_base || pointer >= ga_base + ga_size {
                    continue;
                }
                let rva = pointer - ga_base;
                if !method_addrs.insert(rva) {
                    continue;
                }

                let method_name = il2cpp.method_get_name(method);
                let ret_type = il2cpp.type_get_name(il2cpp.method_get_return_type(method));
                let param_count = il2cpp.method_get_param_count(method);
                let mut params = Vec::new();
                for p in 0..param_count {
                    let ptype = il2cpp.type_get_name(il2cpp.method_get_param(method, p));
                    let pname = il2cpp.method_get_param_name(method, p);
                    params.push(format!("{} {}", ptype, pname));
                }
                let signature = format!("{} {}::{}({})", ret_type, type_prefix, method_name, params.join(", "));

                let unique_name = if duplicates.contains_key(&signature) {
                    let count = duplicates.get_mut(&signature).unwrap();
                    *count += 1;
                    format!("{}_{}", signature, count)
                } else {
                    duplicates.insert(signature.clone(), 0);
                    signature
                };

                method_entries.push(json!({
                    "Address": rva,
                    "Name": unique_name,
                }));
            }
        }
    }

    // Write il2cpp.json
    debug!("{} methods collected", method_entries.len());
    let out_path = out_dir.join("il2cpp.json");
    let f = BufWriter::new(File::create(&out_path).unwrap());

    let root = json!({
        "ScriptMethod": method_entries,
        "ScriptString": string_entries,
        "ScriptField": field_entries,
    });

    serde_json::to_writer_pretty(f, &root).unwrap();
    info!("written {}", out_path.display());
}

// ── public entry point ──────────────────────────────────────────────────────

pub fn dump(out_dir: &Path) {
    dump_cs(out_dir);
    dump_json(out_dir);
}

use crate::module::Module;

pub fn format_rva(addr: usize, modules: &[(&str, &Module)]) -> String {
    for &(name, module) in modules {
        let base = module.handle as usize;
        let end = base + module.size;
        if addr >= base && addr < end {
            return format!("{}!0x{:x}", name, addr - base);
        }
    }
    format!("0x{:x}", addr)
}

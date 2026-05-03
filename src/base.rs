use ctor::ctor;
use log::{error, info};
use std::{fs, thread, time::Duration};
use winapi::um::consoleapi;

use crate::api::get_il2cpp_api;

fn init() {
    thread::sleep(Duration::from_secs(10));

    unsafe { consoleapi::AllocConsole() };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("starting dump");

    let exe_path = std::env::current_exe().unwrap();
    let out_dir = exe_path.parent().unwrap().join("DUMP");
    fs::create_dir_all(&out_dir).unwrap();

    let api = match get_il2cpp_api() {
        Ok(api) => api,
        Err(e) => {
            error!("failed to initialize il2cpp api: {}", e);
            return;
        }
    };
    fs::write(out_dir.join("RVA.txt"), &api.rva_log).unwrap();

    crate::output::dump(&out_dir);

    info!("dump complete");
}

#[ctor]
fn entry() {
    thread::spawn(init);
}

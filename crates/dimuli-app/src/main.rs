//! Desktop entry point: `cargo run -p dimuli-app -- a.jpg b.jpg`.

use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;

use dimuli_app::platform::DesktopPlatform;

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    match dimuli_app::run(Rc::new(DesktopPlatform::new(paths))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!("UI failed: {error}");
            ExitCode::FAILURE
        }
    }
}

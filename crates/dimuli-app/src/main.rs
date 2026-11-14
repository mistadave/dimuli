//! Desktop entry point: `cargo run -p dimuli-app -- a.jpg b.jpg`.

use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;

use dimuli::platform::DesktopPlatform;

fn main() -> ExitCode {
    // The Android build uses the library entry point; the bin only needs to compile there.
    #[cfg(not(target_os = "android"))]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    #[cfg(not(target_os = "android"))]
    select_desktop_backend();
    let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    match dimuli::run(Rc::new(DesktopPlatform::new(paths))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!("UI failed: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Prefers winit with the software renderer on the desktop preview.
///
/// Under WSL2/WSLg the OpenGL (femtovg) renderer leaves the window empty, while the software
/// renderer draws correctly. An explicit `SLINT_BACKEND` always wins.
#[cfg(not(target_os = "android"))]
fn select_desktop_backend() {
    if std::env::var_os("SLINT_BACKEND").is_some() {
        return;
    }
    let result = slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .select();
    if let Err(error) = result {
        log::warn!("software renderer unavailable, using Slint default: {error}");
    }
}

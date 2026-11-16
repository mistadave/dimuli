//! Desktop implementation used for development: images come from the command line.

use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{Platform, PlatformEvent, post_event};
use crate::texts;

/// Desktop platform. "Picking" returns the paths given on the command line
/// (`cargo run -p dimuli-app -- a.jpg b.jpg`); there is no file dialog.
pub struct DesktopPlatform {
    paths: Vec<PathBuf>,
}

impl DesktopPlatform {
    /// Creates the platform with the image paths passed at startup.
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }
}

/// The event answering a pick request: all paths, even more than allowed (the controller cuts and reports).
fn pick_event(paths: &[PathBuf]) -> PlatformEvent {
    if paths.is_empty() {
        PlatformEvent::PickFailed(texts::NO_IMAGE_ARGUMENTS.to_owned())
    } else {
        PlatformEvent::ImagesPicked {
            paths: paths.to_vec(),
            failed: 0,
        }
    }
}

impl Platform for DesktopPlatform {
    fn pick_images(&self, _max: usize) {
        post_event(pick_event(&self.paths));
    }

    fn close_app(&self) {
        if let Err(error) = slint::quit_event_loop() {
            log::error!("cannot quit the event loop: {error}");
        }
    }

    fn save_to_gallery(&self, jpeg: Vec<u8>) {
        let result = save_in_working_dir(&jpeg).map_err(|error| {
            log::error!("cannot save the collage: {error}");
            texts::SAVE_FAILED.to_owned()
        });
        post_event(PlatformEvent::Saved(result));
    }

    fn share_supported(&self) -> bool {
        false
    }

    fn data_dir(&self) -> Option<PathBuf> {
        config_dir(
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("HOME"),
        )
    }

    fn restored_after_close(&self) -> bool {
        false
    }

    fn set_collage_open(&self, _open: bool) {}

    fn share(&self, _jpeg: Vec<u8>) {
        post_event(PlatformEvent::Shared(Err(
            texts::SHARE_UNAVAILABLE.to_owned()
        )));
    }
}

/// `$XDG_CONFIG_HOME/dimuli`, else `$HOME/.config/dimuli`. Relative or empty values are ignored, as the
/// XDG specification requires, so the settings never end up in the working directory.
fn config_dir(xdg_config_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let absolute =
        |value: Option<OsString>| value.map(PathBuf::from).filter(|path| path.is_absolute());
    absolute(xdg_config_home)
        .or_else(|| absolute(home).map(|home| home.join(".config")))
        .map(|base| base.join("dimuli"))
}

/// Writes `jpeg` as `dimuli-<unix seconds>.jpg` into the working directory and logs the full path.
fn save_in_working_dir(jpeg: &[u8]) -> io::Result<()> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let path = write_unique(Path::new("."), seconds, jpeg)?;
    let shown = path.canonicalize().unwrap_or(path);
    log::info!("saved the collage to {}", shown.display());
    Ok(())
}

/// Writes `data` to `dimuli-<seconds>.jpg` in `dir`; if that exists, to `dimuli-<seconds>-1.jpg`, `-2`, ….
/// Runs in the UI thread: acceptable on the desktop (Android saves through Kotlin later).
/// Never overwrites: the file is created with `create_new`, so a race with another writer also just moves on.
fn write_unique(dir: &Path, seconds: u64, data: &[u8]) -> io::Result<PathBuf> {
    for attempt in 0..=MAX_NAME_ATTEMPTS {
        let name = if attempt == 0 {
            format!("dimuli-{seconds}.jpg")
        } else {
            format!("dimuli-{seconds}-{attempt}.jpg")
        };
        let path = dir.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                write_or_remove(file, &path, data)?;
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no free file name",
    ))
}

/// Writes `data` and flushes; on failure removes the partially written file at `path` (logging a failed
/// removal) so no truncated JPEG looks like a valid save. The writer is dropped (closed) before removing.
fn write_or_remove<W: Write>(mut writer: W, path: &Path, data: &[u8]) -> io::Result<()> {
    let result = writer.write_all(data).and_then(|()| writer.flush());
    drop(writer);
    if result.is_err()
        && let Err(remove_error) = std::fs::remove_file(path)
    {
        log::error!(
            "cannot remove partial file {}: {remove_error}",
            path.display()
        );
    }
    result
}

/// How many numbered alternatives are tried before giving up.
const MAX_NAME_ATTEMPTS: u32 = 1000;

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(count: usize) -> Vec<PathBuf> {
        (0..count)
            .map(|i| PathBuf::from(format!("{i}.jpg")))
            .collect()
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("dimuli-desktop-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn saving_uses_timestamp_name_and_never_overwrites() {
        let dir = temp_dir("save");
        let first = write_unique(&dir, 42, b"one").unwrap();
        let second = write_unique(&dir, 42, b"two").unwrap();
        let third = write_unique(&dir, 42, b"three").unwrap();
        assert_eq!(first.file_name().unwrap(), "dimuli-42.jpg");
        assert_eq!(second.file_name().unwrap(), "dimuli-42-1.jpg");
        assert_eq!(third.file_name().unwrap(), "dimuli-42-2.jpg");
        assert_eq!(std::fs::read(&first).unwrap(), b"one");
        assert_eq!(std::fs::read(&second).unwrap(), b"two");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A writer that accepts a few bytes and then fails, like a full disk.
    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("disk full"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn failed_write_removes_the_partial_file() {
        let dir = temp_dir("partial");
        let path = dir.join("partial.jpg");
        std::fs::write(&path, b"par").unwrap();
        assert!(write_or_remove(FailingWriter, &path, b"data").is_err());
        assert!(!path.exists());
        // Removing a file that is already gone only logs.
        assert!(write_or_remove(FailingWriter, &path, b"data").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saving_into_a_missing_directory_fails() {
        let dir = temp_dir("missing").join("nope");
        assert!(write_unique(&dir, 1, b"x").is_err());
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn pick_returns_all_given_paths() {
        assert_eq!(
            pick_event(&paths(2)),
            PlatformEvent::ImagesPicked {
                paths: paths(2),
                failed: 0
            }
        );
        // More than allowed: cutting and telling the user is the controller's job.
        assert_eq!(
            pick_event(&paths(12)),
            PlatformEvent::ImagesPicked {
                paths: paths(12),
                failed: 0
            }
        );
    }

    #[test]
    fn config_dir_prefers_xdg_then_home_and_ignores_relative_paths() {
        let os = |s: &str| Some(OsString::from(s));
        assert_eq!(
            config_dir(os("/x/cfg"), os("/home/u")),
            Some(PathBuf::from("/x/cfg/dimuli"))
        );
        assert_eq!(
            config_dir(None, os("/home/u")),
            Some(PathBuf::from("/home/u/.config/dimuli"))
        );
        assert_eq!(
            config_dir(os(""), os("/home/u")),
            Some(PathBuf::from("/home/u/.config/dimuli"))
        );
        assert_eq!(
            config_dir(os("rel"), os("/home/u")),
            Some(PathBuf::from("/home/u/.config/dimuli"))
        );
        assert_eq!(config_dir(None, None), None);
        assert_eq!(config_dir(None, os("home")), None);
    }

    #[test]
    fn pick_without_arguments_explains_what_to_do() {
        assert!(matches!(
            pick_event(&[]),
            PlatformEvent::PickFailed(message) if message.contains("cargo run")
        ));
    }
}

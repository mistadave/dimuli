//! User-configurable app defaults (`AppSettings`), their text format and their storage on disk.
//!
//! The defaults apply to every new collage (format, spacing, padding, corners, background) and to every
//! export (size, quality). Parsing is tolerant: a damaged or outdated file must never stop the app, so every
//! invalid value falls back to the factory value of its key.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use collage_core::collage::{MAX_CORNER_RADIUS, MAX_PADDING, MAX_SPACING};
use collage_core::{AspectRatio, EXPORT_LONG_EDGE, ExportOptions, Style};

use crate::view::BACKGROUND_PRESETS;

/// Selectable long edges of the exported image in pixels, in display order.
pub(crate) const EXPORT_SIZE_PRESETS: [u32; 4] = [2000, EXPORT_LONG_EDGE, 4000, 6000];
/// Selectable JPEG qualities, in display order.
pub(crate) const JPEG_QUALITY_PRESETS: [u8; 3] = [85, 90, 95];

/// Name of the settings file inside the platform's data directory.
const FILE_NAME: &str = "settings.txt";
/// Temporary name used while writing, so a crash never leaves a half-written settings file.
const TEMP_FILE_NAME: &str = "settings.txt.tmp";
/// Only format version understood by [`AppSettings::parse`].
const FORMAT_VERSION: u32 = 1;
/// Settings files are tiny; anything larger is not ours and is ignored.
const MAX_FILE_BYTES: u64 = 4096;

/// The defaults the user can change. `Default` is the factory configuration.
///
/// Lengths are in collage units (see [`Style`]); the background is an index into [`BACKGROUND_PRESETS`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AppSettings {
    pub(crate) aspect: AspectRatio,
    pub(crate) spacing: f32,
    pub(crate) padding: f32,
    pub(crate) corner_radius: f32,
    background: usize,
    long_edge: u32,
    jpeg_quality: u8,
}

impl Default for AppSettings {
    fn default() -> Self {
        let style = Style::default();
        let export = ExportOptions::default();
        Self {
            aspect: AspectRatio::default(),
            spacing: style.spacing,
            padding: style.padding,
            corner_radius: style.corner_radius,
            // The factory background (white) is the first preset.
            background: BACKGROUND_PRESETS
                .iter()
                .position(|&color| color == style.background)
                .unwrap_or(0),
            long_edge: export.long_edge,
            jpeg_quality: export.jpeg_quality,
        }
    }
}

/// Clamps a length to `0..=max`; NaN becomes `0.0` (like the core does for styles).
fn clamp_length(value: f32, max: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, max)
    }
}

/// Stable name of a format in the settings file (independent of the display order).
fn aspect_name(aspect: AspectRatio) -> &'static str {
    match aspect {
        AspectRatio::Square => "1:1",
        AspectRatio::Portrait4x5 => "4:5",
        AspectRatio::Story9x16 => "9:16",
        AspectRatio::Landscape16x9 => "16:9",
    }
}

/// Parses a finite length in `0..=max`.
fn parse_length(value: &str, max: f32) -> Option<f32> {
    let parsed: f32 = value.parse().ok()?;
    (parsed.is_finite() && (0.0..=max).contains(&parsed)).then_some(parsed)
}

impl AppSettings {
    /// Style for a new collage.
    pub(crate) fn style(&self) -> Style {
        Style {
            spacing: self.spacing,
            padding: self.padding,
            corner_radius: self.corner_radius,
            background: self.background_color(),
        }
    }

    /// Export options for the next export.
    pub(crate) fn export_options(&self) -> ExportOptions {
        ExportOptions {
            long_edge: self.long_edge,
            jpeg_quality: self.jpeg_quality,
        }
    }

    fn background_color(&self) -> collage_core::Rgba {
        BACKGROUND_PRESETS
            .get(self.background)
            .copied()
            .unwrap_or(collage_core::Rgba::WHITE)
    }

    /// Index into [`AspectRatio::ALL`] (for the settings UI); `None` cannot happen for valid settings.
    pub(crate) fn aspect_index(&self) -> Option<usize> {
        AspectRatio::ALL.iter().position(|&a| a == self.aspect)
    }

    /// Index into [`BACKGROUND_PRESETS`].
    pub(crate) fn background_index(&self) -> usize {
        self.background
    }

    /// Index into [`EXPORT_SIZE_PRESETS`]; `None` cannot happen for valid settings.
    pub(crate) fn export_size_index(&self) -> Option<usize> {
        EXPORT_SIZE_PRESETS
            .iter()
            .position(|&e| e == self.long_edge)
    }

    /// Index into [`JPEG_QUALITY_PRESETS`]; `None` cannot happen for valid settings.
    pub(crate) fn jpeg_quality_index(&self) -> Option<usize> {
        JPEG_QUALITY_PRESETS
            .iter()
            .position(|&q| q == self.jpeg_quality)
    }

    // --- setters: each returns whether the value changed (and the settings must be saved) ---

    /// Sets the format by index into [`AspectRatio::ALL`]; invalid indices change nothing.
    pub(crate) fn set_aspect_index(&mut self, index: i32) -> bool {
        let aspect = usize::try_from(index)
            .ok()
            .and_then(|i| AspectRatio::ALL.get(i).copied());
        match aspect {
            Some(aspect) if aspect != self.aspect => {
                self.aspect = aspect;
                true
            }
            _ => false,
        }
    }

    /// Sets the spacing (clamped to the editor's range).
    pub(crate) fn set_spacing(&mut self, value: f32) -> bool {
        replace(&mut self.spacing, clamp_length(value, MAX_SPACING))
    }

    /// Sets the padding (clamped to the editor's range).
    pub(crate) fn set_padding(&mut self, value: f32) -> bool {
        replace(&mut self.padding, clamp_length(value, MAX_PADDING))
    }

    /// Sets the corner radius (clamped to the editor's range).
    pub(crate) fn set_corner_radius(&mut self, value: f32) -> bool {
        replace(
            &mut self.corner_radius,
            clamp_length(value, MAX_CORNER_RADIUS),
        )
    }

    /// Sets the background by index into [`BACKGROUND_PRESETS`]; invalid indices change nothing.
    pub(crate) fn set_background_index(&mut self, index: i32) -> bool {
        match usize::try_from(index) {
            Ok(i) if i < BACKGROUND_PRESETS.len() => replace(&mut self.background, i),
            _ => false,
        }
    }

    /// Sets the export size by index into [`EXPORT_SIZE_PRESETS`]; invalid indices change nothing.
    pub(crate) fn set_export_size_index(&mut self, index: i32) -> bool {
        let value = usize::try_from(index)
            .ok()
            .and_then(|i| EXPORT_SIZE_PRESETS.get(i).copied());
        value.is_some_and(|v| replace(&mut self.long_edge, v))
    }

    /// Sets the JPEG quality by index into [`JPEG_QUALITY_PRESETS`]; invalid indices change nothing.
    pub(crate) fn set_jpeg_quality_index(&mut self, index: i32) -> bool {
        let value = usize::try_from(index)
            .ok()
            .and_then(|i| JPEG_QUALITY_PRESETS.get(i).copied());
        value.is_some_and(|v| replace(&mut self.jpeg_quality, v))
    }

    /// Back to the factory values; returns whether anything changed.
    pub(crate) fn reset(&mut self) -> bool {
        replace(self, Self::default())
    }

    // --- text format ---

    /// Serializes to the versioned `key=value` format (one pair per line).
    pub(crate) fn to_text(self) -> String {
        format!(
            "version={FORMAT_VERSION}\naspect={}\nspacing={}\npadding={}\ncorner_radius={}\n\
             background={}\nexport_long_edge={}\njpeg_quality={}\n",
            aspect_name(self.aspect),
            self.spacing,
            self.padding,
            self.corner_radius,
            self.background,
            self.long_edge,
            self.jpeg_quality,
        )
    }

    /// Parses settings text. Never fails: a missing or unknown version yields the factory settings; unknown keys
    /// are ignored and invalid or out-of-range values fall back to the factory value of that key.
    pub(crate) fn parse(text: &str) -> Self {
        let mut settings = Self::default();
        let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
        let version_ok = lines
            .next()
            .and_then(|line| line.strip_prefix("version="))
            .and_then(|v| v.trim().parse::<u32>().ok())
            == Some(FORMAT_VERSION);
        if !version_ok {
            return settings;
        }
        for line in lines {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            settings.apply(key.trim(), value.trim());
        }
        settings
    }

    /// Applies one `key=value` pair; invalid values leave the (factory) value untouched.
    fn apply(&mut self, key: &str, value: &str) {
        match key {
            "aspect" => {
                if let Some(&aspect) = AspectRatio::ALL.iter().find(|&&a| aspect_name(a) == value) {
                    self.aspect = aspect;
                }
            }
            "spacing" => {
                if let Some(v) = parse_length(value, MAX_SPACING) {
                    self.spacing = v;
                }
            }
            "padding" => {
                if let Some(v) = parse_length(value, MAX_PADDING) {
                    self.padding = v;
                }
            }
            "corner_radius" => {
                if let Some(v) = parse_length(value, MAX_CORNER_RADIUS) {
                    self.corner_radius = v;
                }
            }
            "background" => {
                if let Ok(i) = value.parse::<usize>()
                    && i < BACKGROUND_PRESETS.len()
                {
                    self.background = i;
                }
            }
            "export_long_edge" => {
                if let Ok(v) = value.parse::<u32>()
                    && EXPORT_SIZE_PRESETS.contains(&v)
                {
                    self.long_edge = v;
                }
            }
            "jpeg_quality" => {
                if let Ok(v) = value.parse::<u8>()
                    && JPEG_QUALITY_PRESETS.contains(&v)
                {
                    self.jpeg_quality = v;
                }
            }
            _ => {}
        }
    }
}

/// Assigns `value` and reports whether it differed.
fn replace<T: PartialEq>(slot: &mut T, value: T) -> bool {
    if *slot == value {
        false
    } else {
        *slot = value;
        true
    }
}

/// Loads the settings from `dir`. A missing, oversized or unreadable file means factory settings
/// (logged, never shown to the user).
pub(crate) fn load(dir: &Path) -> AppSettings {
    let path = dir.join(FILE_NAME);
    match fs::metadata(&path) {
        Ok(meta) if meta.len() > MAX_FILE_BYTES => {
            log::warn!(
                "settings file {} is too large, using defaults",
                path.display()
            );
            return AppSettings::default();
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            log::info!("no settings file yet, using defaults");
            return AppSettings::default();
        }
        Err(error) => {
            log::warn!("cannot read settings {}: {error}", path.display());
            return AppSettings::default();
        }
    }
    match fs::read_to_string(&path) {
        Ok(text) => AppSettings::parse(&text),
        Err(error) => {
            log::warn!("cannot read settings {}: {error}", path.display());
            AppSettings::default()
        }
    }
}

/// Writes the settings to `dir` atomically: temp file, flush to disk, rename over the real file.
pub(crate) fn save(dir: &Path, settings: &AppSettings) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let temp = dir.join(TEMP_FILE_NAME);
    let result = write_temp(&temp, settings.to_text().as_bytes())
        .and_then(|()| fs::rename(&temp, dir.join(FILE_NAME)));
    if result.is_err() {
        // Best effort: a leftover temp file is overwritten by the next save anyway.
        let _ = fs::remove_file(&temp);
    }
    result
}

fn write_temp(path: &Path, data: &[u8]) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(data)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-6;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    fn custom() -> AppSettings {
        AppSettings {
            aspect: AspectRatio::Story9x16,
            spacing: 0.0125,
            padding: 0.05,
            corner_radius: 0.07,
            background: 4,
            long_edge: 6000,
            jpeg_quality: 95,
        }
    }

    #[test]
    fn factory_defaults_match_the_built_in_values() {
        let s = AppSettings::default();
        assert_eq!(s.aspect, AspectRatio::Square);
        assert_eq!(s.style(), Style::default());
        assert_eq!(s.export_options(), ExportOptions::default());
        assert_eq!(s.export_size_index(), Some(1));
        assert_eq!(s.jpeg_quality_index(), Some(1));
        assert_eq!(s.background_index(), 0);
        assert_eq!(s.aspect_index(), Some(0));
    }

    #[test]
    fn presets_contain_the_factory_values() {
        assert!(EXPORT_SIZE_PRESETS.contains(&ExportOptions::default().long_edge));
        assert!(JPEG_QUALITY_PRESETS.contains(&ExportOptions::default().jpeg_quality));
    }

    #[test]
    fn text_round_trips() {
        for settings in [AppSettings::default(), custom()] {
            assert_eq!(AppSettings::parse(&settings.to_text()), settings);
        }
    }

    #[test]
    fn every_aspect_background_size_and_quality_round_trips() {
        for aspect in AspectRatio::ALL {
            let s = AppSettings { aspect, ..custom() };
            assert_eq!(AppSettings::parse(&s.to_text()), s);
        }
        for background in 0..BACKGROUND_PRESETS.len() {
            let s = AppSettings {
                background,
                ..custom()
            };
            assert_eq!(AppSettings::parse(&s.to_text()), s);
        }
        for long_edge in EXPORT_SIZE_PRESETS {
            let s = AppSettings {
                long_edge,
                ..custom()
            };
            assert_eq!(AppSettings::parse(&s.to_text()), s);
        }
        for jpeg_quality in JPEG_QUALITY_PRESETS {
            let s = AppSettings {
                jpeg_quality,
                ..custom()
            };
            assert_eq!(AppSettings::parse(&s.to_text()), s);
        }
    }

    #[test]
    fn extreme_lengths_round_trip() {
        let s = AppSettings {
            spacing: MAX_SPACING,
            padding: 0.0,
            corner_radius: MAX_CORNER_RADIUS,
            ..custom()
        };
        assert_eq!(AppSettings::parse(&s.to_text()), s);
    }

    #[test]
    fn empty_and_garbage_text_gives_factory_settings() {
        for text in [
            "",
            "   \n\n",
            "garbage",
            "\u{0}\u{1}\u{2}",
            "=",
            "===\n=\n",
            "version",
        ] {
            assert_eq!(AppSettings::parse(text), AppSettings::default(), "{text:?}");
        }
    }

    #[test]
    fn unknown_or_missing_version_gives_factory_settings() {
        for text in [
            "version=2\nspacing=0.02\n",
            "version=0\nspacing=0.02\n",
            "version=x\nspacing=0.02\n",
            "spacing=0.02\nversion=1\n",
            "spacing=0.02\n",
        ] {
            assert_eq!(AppSettings::parse(text), AppSettings::default(), "{text:?}");
        }
    }

    #[test]
    fn partial_files_keep_factory_values_for_missing_keys() {
        let s = AppSettings::parse("version=1\njpeg_quality=95\n");
        assert_eq!(s.jpeg_quality, 95);
        assert_eq!(
            AppSettings {
                jpeg_quality: 90,
                ..s
            },
            AppSettings::default()
        );
    }

    #[test]
    fn unknown_keys_and_lines_without_equals_are_ignored() {
        let s = AppSettings::parse("version=1\nfuture=1\nno equals here\n  spacing = 0.02 \n");
        assert!(close(s.spacing, 0.02));
        assert_eq!(AppSettings { spacing: 0.0, ..s }, AppSettings::default());
    }

    #[test]
    fn invalid_values_fall_back_per_key_without_affecting_others() {
        let text = "version=1\naspect=3:2\nspacing=NaN\npadding=inf\ncorner_radius=-0.1\n\
                    background=99\nexport_long_edge=2500\njpeg_quality=100\n";
        assert_eq!(AppSettings::parse(text), AppSettings::default());
        let text = "version=1\nspacing=abc\npadding=0.05\nbackground=-1\ncorner_radius=9\n\
                    export_long_edge=4000\njpeg_quality=300\naspect=\n";
        let s = AppSettings::parse(text);
        assert!(close(s.padding, 0.05));
        assert_eq!(s.export_options().long_edge, 4000);
        assert_eq!(s.spacing, AppSettings::default().spacing);
        assert_eq!(s.corner_radius, AppSettings::default().corner_radius);
        assert_eq!(s.background, 0);
        assert_eq!(s.jpeg_quality, 90);
    }

    #[test]
    fn out_of_range_lengths_are_rejected_not_clamped() {
        let s = AppSettings::parse(&format!("version=1\nspacing={}\n", MAX_SPACING * 2.0));
        assert!(close(s.spacing, AppSettings::default().spacing));
    }

    #[test]
    fn setters_clamp_validate_and_report_changes() {
        let mut s = AppSettings::default();
        assert!(!s.set_spacing(s.spacing));
        assert!(s.set_spacing(10.0));
        assert!(close(s.spacing, MAX_SPACING));
        assert!(s.set_padding(f32::NAN));
        assert!(close(s.padding, 0.0));
        assert!(!s.set_corner_radius(-5.0)); // already 0: clamped value is unchanged
        assert!(close(s.corner_radius, 0.0));
        assert!(s.set_corner_radius(f32::INFINITY));
        assert!(close(s.corner_radius, MAX_CORNER_RADIUS));

        assert!(s.set_aspect_index(3));
        assert_eq!(s.aspect, AspectRatio::Landscape16x9);
        assert!(!s.set_aspect_index(3));
        assert!(!s.set_aspect_index(4));
        assert!(!s.set_aspect_index(-1));

        assert!(s.set_background_index(5));
        assert!(!s.set_background_index(6));
        assert!(!s.set_background_index(-1));
        assert_eq!(s.background_index(), 5);

        assert!(s.set_export_size_index(3));
        assert_eq!(s.export_options().long_edge, 6000);
        assert!(!s.set_export_size_index(4));
        assert!(s.set_jpeg_quality_index(0));
        assert_eq!(s.export_options().jpeg_quality, 85);
        assert!(!s.set_jpeg_quality_index(3));
        assert!(!s.set_jpeg_quality_index(i32::MIN));
    }

    #[test]
    fn reset_restores_factory_values_and_reports_change() {
        let mut s = custom();
        assert!(s.reset());
        assert_eq!(s, AppSettings::default());
        assert!(!s.reset());
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("dimuli-settings-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn save_then_load_round_trips_and_leaves_no_temp_file() {
        let dir = temp_dir("roundtrip").join("nested");
        save(&dir, &custom()).unwrap();
        assert_eq!(load(&dir), custom());
        assert!(!dir.join(TEMP_FILE_NAME).exists());
        // Overwriting works too.
        save(&dir, &AppSettings::default()).unwrap();
        assert_eq!(load(&dir), AppSettings::default());
        let _ = fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn load_of_missing_dir_or_file_gives_factory_settings() {
        let dir = temp_dir("missing");
        assert_eq!(load(&dir), AppSettings::default());
    }

    #[test]
    fn load_ignores_oversized_or_non_utf8_files() {
        let dir = temp_dir("bad");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(FILE_NAME), vec![b'a'; 10_000]).unwrap();
        assert_eq!(load(&dir), AppSettings::default());
        fs::write(dir.join(FILE_NAME), [0xff, 0xfe, 0xfd]).unwrap();
        assert_eq!(load(&dir), AppSettings::default());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_into_an_unwritable_location_fails() {
        let dir = temp_dir("blocked");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("file");
        fs::write(&file, b"x").unwrap();
        // A directory below a regular file cannot be created.
        assert!(save(&file.join("sub"), &custom()).is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}

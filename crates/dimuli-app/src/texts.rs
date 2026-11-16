//! User-visible texts produced by Rust code (German). Texts of the `.slint` files live in `ui/texts.slint`.
//! Messages say what happened and what the user can do, without technical details.

pub(crate) const LOADING: &str = "Bilder werden geladen …";
pub(crate) const RESTARTED: &str = "Die App wurde vom System geschlossen. Die Collage konnte nicht erhalten bleiben. Wähle die Bilder bitte erneut aus.";
pub(crate) const NO_IMAGES_LOADED: &str =
    "Die Bilder konnten nicht geladen werden. Bitte wähle andere Dateien aus.";
pub(crate) const GENERIC_ERROR: &str =
    "Das hat leider nicht geklappt. Bitte versuche es noch einmal.";
pub(crate) const EXPORTING: &str = "Collage wird erstellt …";
pub(crate) const EXPORT_SOURCE_UNREADABLE: &str = "Ein Bild der Collage konnte nicht gelesen werden. Bitte prüfe, ob die Datei noch vorhanden ist, oder starte mit neuen Bildern.";
pub(crate) const NO_IMAGE_ARGUMENTS: &str = "Keine Bilder angegeben. Starte die App mit Dateipfaden, z. B.: cargo run -p dimuli-app -- a.jpg b.png";
pub(crate) const SAVE_FAILED: &str =
    "Die Collage konnte nicht gespeichert werden. Bitte versuche es noch einmal.";
// Only used by the Android platform (and its host tests) until the desktop needs them.
#[cfg(any(target_os = "android", test))]
pub(crate) const PICK_FAILED: &str =
    "Die Bildauswahl konnte nicht geöffnet werden. Bitte versuche es noch einmal.";
#[cfg(any(target_os = "android", test))]
pub(crate) const SHARE_FAILED: &str =
    "Die Collage konnte nicht geteilt werden. Bitte versuche es noch einmal.";
pub(crate) const SHARE_UNAVAILABLE: &str =
    "Teilen ist auf diesem Gerät nicht verfügbar. Speichere die Collage stattdessen.";
pub(crate) const SETTINGS_SAVE_FAILED: &str = "Die Einstellungen konnten nicht gespeichert werden. \
     Sie gelten nur, bis die App geschlossen wird.";
pub(crate) const SAVED: &str = "Collage gespeichert.";

/// Notice after some of the chosen files could not be read.
pub(crate) fn images_skipped(count: usize) -> String {
    if count == 1 {
        "Ein Bild konnte nicht gelesen werden und wurde übersprungen. Wähle bei Bedarf ein anderes Bild aus."
            .to_owned()
    } else {
        format!(
            "{count} Bilder konnten nicht gelesen werden und wurden übersprungen. Wähle bei Bedarf andere Bilder aus."
        )
    }
}

/// Notice (or error, if nothing else was usable) after images exceeded the size limits.
pub(crate) fn images_too_large(count: usize) -> String {
    if count == 1 {
        "Ein Bild ist zu groß und wurde übersprungen. Wähle ein kleineres Bild aus.".to_owned()
    } else {
        format!("{count} Bilder sind zu groß und wurden übersprungen. Wähle kleinere Bilder aus.")
    }
}

/// Notice when more images were chosen than fit into a collage.
pub(crate) fn too_many_images(max: usize) -> String {
    format!("Es werden nur die ersten {max} Bilder verwendet.")
}

/// The message for a failed export: names the cause only when the user can act on it.
pub(crate) fn export_error(error: &collage_core::CoreError) -> &'static str {
    use collage_core::CoreError;
    match error {
        CoreError::MissingSource(_)
        | CoreError::ImageDecode { .. }
        | CoreError::ImageTooLarge { .. }
        | CoreError::InvalidImageSize { .. } => EXPORT_SOURCE_UNREADABLE,
        _ => GENERIC_ERROR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collage_core::{CoreError, ImageId};

    #[test]
    fn unreadable_sources_get_a_specific_export_message() {
        assert_eq!(
            export_error(&CoreError::MissingSource(ImageId(0))),
            EXPORT_SOURCE_UNREADABLE
        );
        assert_eq!(export_error(&CoreError::Encode("x".into())), GENERIC_ERROR);
    }

    #[test]
    fn failure_texts_say_what_happened_and_what_to_do() {
        let texts = [
            NO_IMAGES_LOADED.to_owned(),
            RESTARTED.to_owned(),
            GENERIC_ERROR.to_owned(),
            EXPORT_SOURCE_UNREADABLE.to_owned(),
            SAVE_FAILED.to_owned(),
            SETTINGS_SAVE_FAILED.to_owned(),
            PICK_FAILED.to_owned(),
            SHARE_FAILED.to_owned(),
            SHARE_UNAVAILABLE.to_owned(),
            images_skipped(1),
            images_skipped(3),
            images_too_large(1),
            images_too_large(2),
        ];
        for text in &texts {
            let sentences = text.matches(". ").count() + usize::from(text.ends_with('.'));
            assert!(sentences >= 2, "needs two sentences: {text}");
            for marker in ["Error", "error", "panic", "JNI", "Exception", "{"] {
                assert!(!text.contains(marker), "technical marker in: {text}");
            }
        }
    }
}

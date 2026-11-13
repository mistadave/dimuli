//! User-visible texts produced by Rust code (German). Texts of the `.slint` files live in `ui/texts.slint`.
//! Messages say what happened and what the user can do, without technical details.

pub(crate) const LOADING: &str = "Bilder werden geladen …";
pub(crate) const NO_IMAGES_LOADED: &str =
    "Die Bilder konnten nicht geladen werden. Bitte wähle andere Dateien aus.";
pub(crate) const GENERIC_ERROR: &str =
    "Das hat leider nicht geklappt. Bitte versuche es noch einmal.";
pub(crate) const EXPORTING: &str = "Collage wird erstellt …";
pub(crate) const EXPORT_SOURCE_UNREADABLE: &str = "Ein Bild der Collage konnte nicht gelesen werden. Bitte prüfe, ob die Datei noch vorhanden ist, oder starte mit neuen Bildern.";
pub(crate) const NO_IMAGE_ARGUMENTS: &str = "Keine Bilder angegeben. Starte die App mit Dateipfaden, z. B.: cargo run -p dimuli-app -- a.jpg b.png";
pub(crate) const SAVE_FAILED: &str =
    "Die Collage konnte nicht gespeichert werden. Bitte versuche es noch einmal.";
pub(crate) const SHARE_UNAVAILABLE: &str = "Teilen ist auf diesem Gerät nicht verfügbar.";
pub(crate) const SAVED: &str = "Collage gespeichert.";
pub(crate) const SHARED: &str = "Collage geteilt.";

/// Notice after some of the chosen files could not be read.
pub(crate) fn images_skipped(count: usize) -> String {
    if count == 1 {
        "Ein Bild konnte nicht geladen werden und wurde übersprungen.".to_owned()
    } else {
        format!("{count} Bilder konnten nicht geladen werden und wurden übersprungen.")
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
}

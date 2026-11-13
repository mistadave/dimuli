//! User-visible texts produced by Rust code (German). Texts of the `.slint` files live in `ui/texts.slint`.
//! Messages say what happened and what the user can do, without technical details.

pub(crate) const LOADING: &str = "Bilder werden geladen …";
pub(crate) const NO_IMAGES_LOADED: &str =
    "Die Bilder konnten nicht geladen werden. Bitte wähle andere Dateien aus.";
pub(crate) const GENERIC_ERROR: &str =
    "Das hat leider nicht geklappt. Bitte versuche es noch einmal.";
pub(crate) const NO_IMAGE_ARGUMENTS: &str = "Keine Bilder angegeben. Starte die App mit Dateipfaden, z. B.: cargo run -p dimuli-app -- a.jpg b.png";
pub(crate) const EXPORT_UNAVAILABLE: &str = "Speichern und Teilen sind noch nicht verfügbar.";
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

use std::sync::{Arc, OnceLock};

use resvg::usvg::fontdb::Database;

/// The system fonts, loaded once and shared by every SVG render.
pub fn system_fonts() -> Arc<Database> {
    static FONTS: OnceLock<Arc<Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut database = Database::new();
            database.load_system_fonts();
            Arc::new(database)
        })
        .clone()
}

pub const MONOSPACE_FAMILY: &str = "monospace";

/// The user's monospace font as fontconfig resolves it, for registering with the window.
pub fn monospace_font() -> Option<Vec<u8>> {
    let output = std::process::Command::new("fc-match")
        .args(["--format=%{file}", MONOSPACE_FAMILY])
        .output()
        .ok()?;
    let path = String::from_utf8(output.stdout).ok()?;
    std::fs::read(path.trim()).ok()
}

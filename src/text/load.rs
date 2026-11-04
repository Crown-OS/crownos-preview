use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::Result;

const MAX_TEXT_BYTES: u64 = 2 * 1024 * 1024;

pub struct Loaded {
    pub text: String,
    pub truncated: bool,
}

/// At most the first 2 MiB of `path` as text, invalid UTF-8 replaced.
pub fn read_text(path: &Path) -> Result<Loaded> {
    let file = File::open(path)?;
    let truncated = file.metadata()?.len() > MAX_TEXT_BYTES;
    let mut bytes = Vec::new();
    file.take(MAX_TEXT_BYTES).read_to_end(&mut bytes)?;
    let text = String::from_utf8(bytes)
        .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned());
    Ok(Loaded { text, truncated })
}

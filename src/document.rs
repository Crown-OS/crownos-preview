use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocumentKind {
    Raster,
    Vector,
    Heif,
    Pdf,
    Video,
    Audio,
    Markdown,
    Code,
    PlainText,
}

const CODE_EXTENSIONS: &str = "rs py js mjs ts tsx jsx c h cc cpp hpp cs go java kt swift rb php \
    lua sh bash zsh fish toml json yaml yml xml html htm css scss sql ron ini conf diff patch \
    makefile cmake dockerfile zig hs ml ex exs erl clj scala pl r";

const PLAIN_TEXT_EXTENSIONS: &str = "txt log csv tsv text nfo srt";

fn listed(extensions: &str, extension: &str) -> bool {
    extensions
        .split_whitespace()
        .any(|listed| listed == extension)
}

impl DocumentKind {
    pub fn of(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        let kind = match extension.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "tif" | "tiff" | "bmp" | "ico" => {
                Self::Raster
            }
            "svg" => Self::Vector,
            "heic" | "heif" | "avif" => Self::Heif,
            "pdf" => Self::Pdf,
            "mp4" | "m4v" | "mkv" | "webm" | "mov" | "avi" | "ogv" => Self::Video,
            "mp3" | "flac" | "ogg" | "oga" | "opus" | "wav" | "m4a" | "aac" => Self::Audio,
            "md" | "markdown" => Self::Markdown,
            other if listed(CODE_EXTENSIONS, other) => Self::Code,
            other if listed(PLAIN_TEXT_EXTENSIONS, other) => Self::PlainText,
            _ => return None,
        };
        Some(kind)
    }

    pub fn is_editable(self) -> bool {
        matches!(self, Self::Raster | Self::Heif)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_follow_the_extension_case_insensitively() {
        let kind = |name: &str| DocumentKind::of(Path::new(name));
        assert_eq!(kind("shot.PNG"), Some(DocumentKind::Raster));
        assert_eq!(kind("a.heic"), Some(DocumentKind::Heif));
        assert_eq!(kind("paper.pdf"), Some(DocumentKind::Pdf));
        assert_eq!(kind("clip.webm"), Some(DocumentKind::Video));
        assert_eq!(kind("README.md"), Some(DocumentKind::Markdown));
        assert_eq!(kind("main.rs"), Some(DocumentKind::Code));
        assert_eq!(kind("notes.txt"), Some(DocumentKind::PlainText));
        assert_eq!(kind("archive.zip"), None);
        assert_eq!(kind("Makefile"), None);
    }
}

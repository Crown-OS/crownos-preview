use std::path::PathBuf;

use crate::document::DocumentKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    View,
    Edit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub path: PathBuf,
    pub mode: Mode,
}

impl Request {
    pub fn new(path: impl Into<PathBuf>, mode: Mode) -> Option<Self> {
        let path = path.into();
        let previewable = path.is_file() && DocumentKind::of(&path).is_some();
        previewable.then_some(Self { path, mode })
    }

    pub fn from_args(args: impl IntoIterator<Item = std::ffi::OsString>) -> Option<Self> {
        let mut mode = Mode::View;
        let mut path = None;
        for arg in args {
            if arg == "--edit" {
                mode = Mode::Edit;
            } else {
                path = Some(PathBuf::from(arg));
            }
        }
        Self::new(path?, mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_flag_is_read_in_any_position() {
        let manifest = env!("CARGO_MANIFEST_DIR").to_owned() + "/Cargo.toml";
        let args = |list: &[&str]| Request::from_args(list.iter().map(Into::into));
        assert_eq!(
            args(&["--edit", &manifest]).map(|r| r.mode),
            Some(Mode::Edit)
        );
        assert_eq!(
            args(&[&manifest, "--edit"]).map(|r| r.mode),
            Some(Mode::Edit)
        );
        assert_eq!(args(&[&manifest]).map(|r| r.mode), Some(Mode::View));
        assert_eq!(args(&["--edit"]), None);
        assert_eq!(args(&["/no/such/file.png"]), None);
    }
}

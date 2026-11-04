use std::path::{Path, PathBuf};

use crate::document::DocumentKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Previous,
    Next,
}

/// The previewable file beside `path` in its folder, in name order, wrapping around.
pub fn sibling(path: &Path, direction: Direction) -> Option<PathBuf> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(path.parent()?)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|candidate| candidate.is_file() && DocumentKind::of(candidate).is_some())
        .collect();
    entries.sort_unstable();
    let index = entries.iter().position(|candidate| candidate == path)?;
    let count = entries.len();
    let target = match direction {
        Direction::Previous => (index + count - 1) % count,
        Direction::Next => (index + 1) % count,
    };
    (target != index).then(|| entries.swap_remove(target))
}

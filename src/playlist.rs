use std::path::{Path, PathBuf};

use crate::launch::{Mode, Request};
use crate::util::siblings::Direction;

/// The files a caller asked to step through, in the caller's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playlist {
    paths: Vec<PathBuf>,
    index: usize,
}

impl Playlist {
    /// `None` when `index` is out of range or that file cannot be previewed.
    pub fn new(paths: Vec<String>, index: u32) -> Option<Self> {
        let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        let index = usize::try_from(index).ok()?;
        Request::new(paths.get(index)?, Mode::View)?;
        Some(Self { paths, index })
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn current(&self) -> &Path {
        &self.paths[self.index]
    }

    pub fn request(&self) -> Option<Request> {
        Request::new(self.current(), Mode::View)
    }

    /// Moves to `index` when that file can be previewed.
    pub fn select(&mut self, index: usize) -> Option<Request> {
        let request = Request::new(self.paths.get(index)?, Mode::View)?;
        self.index = index;
        Some(request)
    }

    /// Moves to the next previewable file in `direction`, wrapping around.
    pub fn step(&mut self, direction: Direction) -> Option<Request> {
        let count = self.paths.len();
        (1..count).find_map(|distance| {
            let index = match direction {
                Direction::Next => (self.index + distance) % count,
                Direction::Previous => (self.index + count - distance) % count,
            };
            self.select(index)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_dir_file(name: &str) -> String {
        format!("{}/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn steps_in_the_callers_order_skipping_what_cannot_be_previewed() {
        let toml = manifest_dir_file("Cargo.toml");
        let lib = manifest_dir_file("src/lib.rs");
        let missing = manifest_dir_file("missing.png");
        let mut playlist = Playlist::new(vec![lib.clone(), missing, toml.clone()], 0)
            .expect("the first file is previewable");
        let mut step = |direction| playlist.step(direction).map(|request| request.path);

        assert_eq!(step(Direction::Next), Some(PathBuf::from(&toml)));
        assert_eq!(step(Direction::Next), Some(PathBuf::from(&lib)));
        assert_eq!(step(Direction::Previous), Some(PathBuf::from(&toml)));
        assert_eq!(playlist.index(), 2);
    }

    #[test]
    fn rejects_an_index_outside_the_list() {
        assert_eq!(
            Playlist::new(vec![manifest_dir_file("src/lib.rs")], 3),
            None
        );
        assert_eq!(Playlist::new(Vec::new(), 0), None);
    }
}

//! crownos-preview — the CrownOS quick-look previewer.
//!
//! The library target is the wire contract of the `crownos-preview`
//! crownos-ipc service; with `default-features = false` a client builds only
//! that.

crownos_ipc::protocol! {
    pub mod protocol: "crownos-preview" {
        /// Show the file at `path`. Replies `false` when it cannot be previewed.
        fn open(path: String) -> bool;
        /// Show the file at `path` with markup and crop tools open.
        /// Replies `false` when it cannot be previewed.
        fn edit(path: String) -> bool;
        /// Hide the preview window.
        notify close();
        /// The preview of `path` was dismissed.
        event Closed { path: String };
        /// Show `paths[index]`; the arrow keys then step through `paths` in
        /// the caller's order instead of the folder's. Replies `false` when
        /// that file cannot be previewed.
        fn open_list(paths: Vec<String>, index: u32) -> bool;
        /// Show `paths[index]` of the list last given to `open_list`.
        notify select(index: u32);
        /// The viewer stepped to `paths[index]` of the list last given to
        /// `open_list`.
        event Navigated { index: u32, path: String };
    }
}

pub use protocol::{Client, Closed, Navigated, SERVICE};

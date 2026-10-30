//! crownos-preview — the CrownOS quick-look previewer.
//!
//! The library target is the wire contract of the `crownos-preview`
//! crownos-ipc service; with `default-features = false` a client builds only
//! that.

crownos_ipc::protocol! {
    pub mod protocol: "crownos-preview" {
        /// Show the file at `path`. Replies `false` when it cannot be previewed.
        fn open(path: String) -> bool;
        /// Hide the preview window.
        notify close();
        /// The preview of `path` was dismissed.
        event Closed { path: String };
    }
}

pub use protocol::{Client, Closed, SERVICE};

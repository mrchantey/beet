//! `no_std`-capable path types and helpers.
//!
//! Built on [`alloc`] and [`SmolStr`] rather than [`std::path`]: the
//! unopinionated [`SmolPath`] and the three rooted newtypes over it, a
//! [`RelPath`] key within a store, an [`AbsPath`] filesystem location and a
//! [`WsPath`] workspace-relative location, plus the [`Url`] a request is
//! routed by and the [`path_ext::clean`] cleaner. Resolving a path against
//! the current directory or workspace root, and the filesystem
//! [`Path`](std::path::Path) helpers in [`path_ext`], are gated behind `std`;
//! the filesystem itself lives in [`fs_ext`](crate::prelude::fs_ext).

mod abs_path;
pub mod path_ext;
mod rel_path;
mod smol_path;
mod url;
mod ws_path;

pub use abs_path::*;
pub use rel_path::*;
pub use smol_path::*;
pub use url::*;
pub use ws_path::*;

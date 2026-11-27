//! `no_std`-capable path types and helpers.
//!
//! Built on [`alloc`] and [`SmolStr`] rather than [`std::path`]: the
//! unopinionated [`SmolPath`] and the three newtypes over it, a [`FwdPath`]
//! location within a container that never leaves it, an [`AbsPath`] filesystem location and a
//! [`WsPath`] workspace-relative location, plus the [`Url`] a request is
//! routed by, the verbatim [`Uri`] a stored field holds, and the
//! [`path_ext::clean`] cleaner. Resolving a path against
//! the current directory or workspace root, and the filesystem
//! [`Path`](std::path::Path) helpers in [`path_ext`], are gated behind `std`;
//! the filesystem itself lives in [`fs_ext`](crate::prelude::fs_ext).

mod abs_path;
mod doc_path;
mod fwd_path;
pub mod path_ext;
mod smol_path;
mod uri;
mod url;
mod ws_path;

pub use abs_path::*;
pub use doc_path::*;
pub use fwd_path::*;
pub use smol_path::*;
pub use uri::*;
pub use url::*;
pub use ws_path::*;

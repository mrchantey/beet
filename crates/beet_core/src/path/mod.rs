//! `no_std`-capable path types and helpers.
//!
//! Unlike [`path_utils`](crate::path_utils) (filesystem-backed, std-only),
//! the core of this module is built on [`alloc`] and [`SmolStr`]: the logical
//! [`SmolPath`] type, the [`Url`] a request is routed by, and the
//! [`path_ext::clean`] cleaner. The filesystem [`Path`](std::path::Path)
//! helpers in [`path_ext`] are gated behind `std`.

pub mod path_ext;
mod smol_path;
mod url;

pub use smol_path::*;
pub use url::*;

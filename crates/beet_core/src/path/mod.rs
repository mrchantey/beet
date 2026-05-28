//! `no_std`-capable path types.
//!
//! Unlike [`path_utils`](crate::path_utils) (filesystem-backed,
//! std-only), this module contains pure logical path types built on
//! [`alloc`] and [`SmolStr`].

mod smol_path;

pub use smol_path::*;

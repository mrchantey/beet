//! The vocabulary a package is compiled in: what an author declares, what the
//! compiler hoists, and the components beet's records are made of.
//!
//! A package is a directory whose entry document declares [`Package`]; a
//! *branch* is its files ([`Files`], keyed by path), and compiling one turns
//! each document into a scene and writes a package record pointing at them.
//! The records themselves (branch, package, release, scene) and the compiler
//! are `beet_scene`'s; this is what every plugin declares with, so a leaf
//! crate contributes to a record without linking the toolchain:
//!
//! - [`SceneMeta`], type data a component carries to contribute to the
//!   record beside its scene, seeing one document; [`PackageMeta`], to the
//!   package record, seeing the whole branch
//! - [`Package`] and its children [`Export`], [`Dependency`], [`Import`] and
//!   [`Runtime`], hoisted into [`Exports`], [`Dependencies`] and
//!   [`ExportRuntimes`]
//! - the scene meta the compiler writes, [`Imports`] and [`Props`] (and
//!   [`RequireCfg`]'s union), and the use site's [`LoadRoot`] and [`Preload`]
//! - the package meta, [`Branch`], [`SourceBranch`], [`Files`], [`Scenes`]
//!   and [`LoadSets`]
//! - the names, [`PackageName`], [`BranchName`], [`RuntimeName`] and
//!   [`PackageAddress`], versions being beet's one [`Version`] and
//!   [`VersionReq`]
mod branch;
mod names;
#[allow(clippy::module_inception)]
mod package;
mod scene_meta;
mod scenes;
pub use branch::*;
pub use names::*;
pub use package::*;
pub use scene_meta::*;
pub use scenes::*;

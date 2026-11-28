//! The compiler: a branch of a package compiled into scenes and one package
//! record, in the package's space, then frozen as a release and published.
//!
//! Three records for one package, each an entity body keyed by its identity
//! ([`EntityCollection`]), beside the scenes they share:
//!
//! - a **branch record** ([`BranchRecord`], `<package>:<branch>`) is the
//!   working files, rewritten by every edit;
//! - a **package record** ([`PackageRecord`], the same rkey) is that branch
//!   compiled, its scenes and the package meta, stale once the branch moves
//!   past the [`SourceBranch`] it names;
//! - a **release** ([`ReleaseRecord`], `<package>:<version>`) is a package
//!   record frozen under its version, its body copied whole, created and never
//!   rewritten, the newest with no prerelease also copied to `latest`;
//! - a **scene record** ([`SceneRecord`], keyed by its body's cid) is one
//!   document compiled, shared by every branch and release whose document did
//!   not change.
//!
//! Two verbs follow a compile ([`PackageReleaser`]). *Release* freezes a
//! package record in the package's space, so a release may stay private.
//! *Publish* copies a release to the account's public repo byte for byte with
//! every scene and blob it names, so the copy's cid is the space's and the two
//! agree exactly when their cids do; asked to, it also writes the account's
//! [`HomeRecord`].
//!
//! [`PackageCompiler`] writes into the package's space under a source
//! directory's `.beet/pds/`, the layout [`EmulatorPds`] keeps, so a source
//! directory's compiled form reads exactly as a hosted one does. `beet
//! compile`, `beet release` and `beet publish` are the verbs.
//!
//! ```mermaid
//! flowchart TD
//!     files[the package's files] --> lower[each document lowered by its parser]
//!     lower --> resolve[every DocPath resolved package absolute]
//!     resolve --> skeleton[its skeleton in a compile world]
//!     skeleton --> scene[scene meta hoisted: a scene record]
//!     scene --> package[package meta: props, cycles, load sets]
//!     package --> records[branch record and package record]
//!     records --> release[release: the package record frozen in the space]
//!     release --> publish[publish: the release copied to the public repo]
//! ```
mod document;
mod package_compiler;
mod package_meta;
mod package_releaser;
mod records;
mod references;
mod space_writer;
use document::*;
pub use package_compiler::*;
use package_meta::*;
pub use package_releaser::*;
pub use records::*;
use references::*;
pub use space_writer::*;

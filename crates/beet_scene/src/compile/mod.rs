//! The compiler: a branch of a package compiled into scenes and one package
//! record, in the package's space.
//!
//! Three records for one package, each an entity body keyed by its identity
//! ([`EntityCollection`]):
//!
//! - a **branch record** ([`BranchRecord`], `<package>:<branch>`) is the
//!   working files, rewritten by every edit;
//! - a **package record** ([`PackageRecord`], the same rkey) is that branch
//!   compiled, its scenes and the package meta, stale once the branch moves
//!   past the [`SourceBranch`] it names;
//! - a **scene record** ([`SceneRecord`], keyed by its body's cid) is one
//!   document compiled, shared by every branch whose document did not change.
//!
//! [`PackageCompiler`] writes all three into the package's space under a home
//! directory's `.beet/pds/`, the layout [`EmulatorPds`] keeps, so a home
//! directory's compiled form reads exactly as a hosted one does. `beet
//! compile` is the verb.
//!
//! ```mermaid
//! flowchart TD
//!     files[the package's files] --> lower[each document lowered by its parser]
//!     lower --> resolve[every DocPath resolved package absolute]
//!     resolve --> skeleton[its skeleton in a compile world]
//!     skeleton --> scene[scene meta hoisted: a scene record]
//!     scene --> package[package meta: props, cycles, load sets]
//!     package --> records[branch record and package record]
//! ```
mod document;
mod package_compiler;
mod package_meta;
mod records;
mod references;
mod space_writer;
use document::*;
pub use package_compiler::*;
use package_meta::*;
pub use records::*;
use references::*;
pub use space_writer::*;

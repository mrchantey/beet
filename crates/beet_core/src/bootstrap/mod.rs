//! Everything a beet process resolves *before* a scene exists.
//!
//! [`BootstrapConfig`] is the type: every pre-scene knob, parsed from argv and
//! the `BEET_*` environment, readable from anywhere through
//! [`BootstrapConfig::get`]. The rest of this module is either the grammar one
//! of its fields parses into ([`StoreUri`], [`RunningSetFilter`],
//! [`ServiceAccess`]),
//! or [`PackageConfig`], the resource describing *this build* rather than
//! *this launch* (the package's identity, usually from
//! [`pkg_config!`](crate::pkg_config)).

mod bootstrap_config;
mod child_process;
mod running_set_filter;
mod service_access;
mod store_uri;
pub use bootstrap_config::*;
pub use child_process::*;
pub use running_set_filter::*;
pub use service_access::*;
pub use store_uri::*;

// `heck` casing and the workspace path helpers are std-only.
#[cfg(feature = "std")]
mod package_config;
#[cfg(feature = "std")]
pub use package_config::*;

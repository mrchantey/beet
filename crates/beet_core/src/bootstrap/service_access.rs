//! The `--service-access` selection grammar.

use crate::prelude::*;
use core::fmt;
use core::str::FromStr;

/// Whether services resolve locally or against the cloud, parsed once from
/// `--service-access` / `BEET_SERVICE_ACCESS`.
///
/// For instance a bucket that is a local directory during development and an s3
/// bucket when deployed.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let access: ServiceAccess = "remote".parse().unwrap();
/// access.xpect_eq(ServiceAccess::Remote);
/// ```
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Reflect)]
#[reflect(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ServiceAccess {
	/// Services are accessed via the filesystem and local servers.
	#[default]
	Local,
	/// Services are accessed via remote cloud services.
	Remote,
}

impl FromStr for ServiceAccess {
	type Err = String;
	fn from_str(value: &str) -> Result<Self, Self::Err> {
		match value.to_lowercase().as_str() {
			"local" => Ok(ServiceAccess::Local),
			"remote" => Ok(ServiceAccess::Remote),
			other => Err(format!(
				"invalid service access `{other}`, expected `local` or `remote`"
			)),
		}
	}
}

impl fmt::Display for ServiceAccess {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			ServiceAccess::Local => write!(f, "local"),
			ServiceAccess::Remote => write!(f, "remote"),
		}
	}
}

#[cfg(feature = "std")]
impl ServiceAccess {
	/// The workspace directory backing the [`Local`](Self::Local) stand-in for
	/// the resource named `name`, ie `target/stores/beet-site--dev--analytics`.
	/// Keyed by the composed resource name rather than the bare label, so two
	/// apps or two stages in one workspace never share a directory, exactly as
	/// they never share a bucket.
	pub fn local_store_dir(name: impl AsRef<str>) -> WsPath {
		WsPath::new("target/stores").join(name)
	}

	/// The store a [`Local`](Self::Local) process attaches for the resource
	/// named `name`: the absolute [`local_store_dir`](Self::local_store_dir)
	/// on any host with a filesystem (native, deno, node), else (a browser)
	/// one IndexedDB database of that name. The local stand-in for the cloud
	/// resource the same declaration names when [`Remote`](Self::Remote), so
	/// one markup declaration runs both ways, and one directory or database
	/// per declaration either way.
	pub fn local_store_uri(name: &str) -> StoreUri {
		match Self::host_has_fs() {
			true => StoreUri::Fs {
				path_prefix: Some(
					Self::local_store_dir(name).into_abs().into(),
				),
			},
			false => StoreUri::IndexedDb {
				name: name.into(),
				path_prefix: None,
			},
		}
	}

	/// Whether this host reaches a filesystem: every native target, and a js
	/// runtime that has one (deno, node).
	fn host_has_fs() -> bool {
		cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				js_runtime::environment().has_fs()
			} else {
				true
			}
		}
	}
}

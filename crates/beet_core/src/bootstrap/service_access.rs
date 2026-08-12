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

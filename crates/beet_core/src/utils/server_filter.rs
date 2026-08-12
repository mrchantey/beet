//! The `--server` selection grammar.

use crate::prelude::*;
use core::fmt;
use core::str::FromStr;

/// Which servers a boot brings up, parsed once from `--server` / `BEET_SERVER`
/// instead of re-split at each consumer.
///
/// A comma-separated glob list, eg `--server=http,ssh` or `--server=*-tui`. An
/// empty list matches every server, which is what a bare `--server=` means: the
/// selection is present but unconstrained.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let filter: ServerFilter = "http,ssh".parse().unwrap();
/// filter.passes("http").xpect_true();
/// filter.passes("cli").xpect_false();
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[reflect(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ServerFilter(Vec<SmolStr>);

impl ServerFilter {
	/// Parse a comma-separated glob list, trimming each name and dropping empty
	/// ones.
	pub fn new(value: &str) -> Self {
		value
			.split(',')
			.map(str::trim)
			.filter(|name| !name.is_empty())
			.map(SmolStr::from)
			.collect::<Vec<_>>()
			.xmap(Self)
	}

	/// Extend this filter with another comma-separated glob list, so repeated
	/// `--server` flags accumulate.
	pub fn extend(&mut self, value: &str) { self.0.extend(Self::new(value).0); }

	/// Whether `name` is selected. An empty filter passes everything, matching
	/// [`GlobFilter`]'s empty-include semantics.
	pub fn passes(&self, name: &str) -> bool {
		self.0
			.iter()
			.fold(GlobFilter::default(), |filter, glob| {
				filter.with_include(glob.as_str())
			})
			.passes(name)
	}
}

impl FromStr for ServerFilter {
	type Err = core::convert::Infallible;
	fn from_str(value: &str) -> Result<Self, Self::Err> { Ok(Self::new(value)) }
}

impl fmt::Display for ServerFilter {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for (index, name) in self.0.iter().enumerate() {
			if index > 0 {
				write!(f, ",")?;
			}
			write!(f, "{name}")?;
		}
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn round_trips() {
		for value in ["http", "http,ssh", "*-tui,cli", ""] {
			ServerFilter::new(value).to_string().xpect_eq(value);
		}
	}

	/// Names are trimmed and empty entries dropped, so `--server=http, ssh,`
	/// selects the same pair as `--server=http,ssh`.
	#[crate::test]
	fn trims_and_drops_empty() {
		ServerFilter::new("http, ssh,")
			.to_string()
			.xpect_eq("http,ssh");
	}

	/// An empty selection is present but unconstrained, so it passes everything.
	#[crate::test]
	fn empty_passes_all() {
		ServerFilter::default().passes("anything").xpect_true();
	}

	#[crate::test]
	fn globs_match() {
		ServerFilter::new("http,ssh").passes("http").xpect_true();
		ServerFilter::new("http,ssh").passes("cli").xpect_false();
		ServerFilter::new("*-tui").passes("ssh-tui").xpect_true();
	}
}

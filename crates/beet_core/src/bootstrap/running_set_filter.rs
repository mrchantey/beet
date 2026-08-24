//! The `--server` selection grammar.

use crate::prelude::*;
use core::fmt;
use core::str::FromStr;

/// Which of an entity's long-running facets a start brings up, parsed once from
/// `--server` / `BEET_SERVER` instead of re-split at each consumer.
///
/// The grammar a `RunningSet` facet's `select` closure reads: a server is the
/// reference facet, hence the flag's name, but any facet naming itself takes part
/// in the same selection.
///
/// A comma-separated glob list, eg `--server=http,ssh` or `--server=*-tui`. An
/// empty list matches every facet, which is what a bare `--server=` means: the
/// selection is present but unconstrained.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let filter: RunningSetFilter = "http,ssh".parse().unwrap();
/// filter.passes("http").xpect_true();
/// filter.passes("cli").xpect_false();
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[reflect(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RunningSetFilter(Vec<SmolStr>);

impl RunningSetFilter {
	/// The one name this selection travels under: the `--server` flag on a boot
	/// request and, as `BEET_SERVER`, the [`BootstrapConfig`] knob a deploy
	/// renders. Declared once so the deploy's rendered argv and a booting
	/// server's read cannot drift apart.
	pub const PARAM: &'static str = "server";

	/// The selection `params` carry on [`PARAM`](Self::PARAM), accumulated across
	/// repeated flags.
	///
	/// `None` when the flag is absent entirely, which is what leaves each
	/// server's own `default_boot` to decide. A bare `--server` is present but
	/// unconstrained, so it selects every server.
	pub fn from_params(params: &MultiMap<SmolStr, SmolStr>) -> Option<Self> {
		params.get_vec(Self::PARAM).map(|values| {
			values.iter().fold(Self::default(), |mut filter, value| {
				filter.extend(value);
				filter
			})
		})
	}

	/// Whether a server named `name` should boot for a request carrying `params`,
	/// from that request's own filter (`--server`), else the process
	/// [`BootstrapConfig`]'s (`--server`, else `BEET_SERVER`).
	///
	/// The process-config fallback is why a deployed binary launched with no args
	/// (a lambda bootstrap, a lightsail systemd unit) still selects its transport:
	/// its `BEET_SERVER` reaches the boot even though the synthesized request
	/// carries no flag. Absent both, `default_boot` decides: it is `true` for
	/// every built-in server, so a bare `beet` brings up every declared one, and
	/// an entry clears it on a server that should boot only when named.
	pub fn selects(
		params: &MultiMap<SmolStr, SmolStr>,
		name: &str,
		default_boot: bool,
	) -> bool {
		Self::from_params(params)
			.as_ref()
			.or(BootstrapConfig::get().server.as_ref())
			.map(|filter| filter.passes(name))
			.unwrap_or(default_boot)
	}

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

impl FromStr for RunningSetFilter {
	type Err = core::convert::Infallible;
	fn from_str(value: &str) -> Result<Self, Self::Err> { Ok(Self::new(value)) }
}

impl fmt::Display for RunningSetFilter {
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
			RunningSetFilter::new(value).to_string().xpect_eq(value);
		}
	}

	/// Names are trimmed and empty entries dropped, so `--server=http, ssh,`
	/// selects the same pair as `--server=http,ssh`.
	#[crate::test]
	fn trims_and_drops_empty() {
		RunningSetFilter::new("http, ssh,")
			.to_string()
			.xpect_eq("http,ssh");
	}

	/// An empty selection is present but unconstrained, so it passes everything.
	#[crate::test]
	fn empty_passes_all() {
		RunningSetFilter::default().passes("anything").xpect_true();
	}

	#[crate::test]
	fn globs_match() {
		RunningSetFilter::new("http,ssh")
			.passes("http")
			.xpect_true();
		RunningSetFilter::new("http,ssh")
			.passes("cli")
			.xpect_false();
		RunningSetFilter::new("*-tui")
			.passes("ssh-tui")
			.xpect_true();
	}

	/// An absent `--server` is no selection at all (each server's `default_boot`
	/// decides), a bare one is an unconstrained selection, and repeated flags
	/// accumulate.
	#[crate::test]
	fn reads_params() {
		let from = |args: &str| {
			RunningSetFilter::from_params(&CliArgs::parse(args).params)
		};
		from("").xpect_none();
		from("--server").unwrap().passes("http").xpect_true();
		from("--server=http --server=ssh")
			.unwrap()
			.to_string()
			.xpect_eq("http,ssh");
	}

	/// The whole selection decision a facet's `select` closure makes, pinned end
	/// to end: `--server` names the facets that take part, and absent the flag the
	/// facet's own `default_boot` decides.
	#[crate::test]
	fn selects_reads_the_filter() {
		let selects = |args: &str, name: &str, default_boot: bool| {
			RunningSetFilter::selects(
				&CliArgs::parse(args).params,
				name,
				default_boot,
			)
		};
		selects("--server=http,ssh", "http", false).xpect_true();
		selects("--server=http", "cli", true).xpect_false();
		// a bare `--server` is present but unconstrained
		selects("--server", "cli", false).xpect_true();
		// absent, the facet's own `default_boot` decides
		selects("", "http", true).xpect_true();
		selects("", "http", false).xpect_false();
	}
}

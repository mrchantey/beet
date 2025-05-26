use rmcp::schemars;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::GlobFilter;


#[derive(
	Debug,
	Default,
	Copy,
	Clone,
	Hash,
	PartialEq,
	Eq,
	Serialize,
	Deserialize,
	schemars::JsonSchema,
)]
pub enum CrateQueryScope {
	#[default]
	#[schemars(description = "\
How to use the crate, ie examples, tests, documentation. \
This should be the default scope for most queries.")]
	PublicApi,
	#[schemars(description = "\
Implementations of engine internals. \
Only query for this if you are certain you need to know how the internals of a function\
as it may misguide you to reimplement engine internals instead of using the public API. \
an example for an acceptable use is implementing new features for the crate
	 ")]
	Internals,
}

impl CrateQueryScope {
	pub fn filter(&self) -> GlobFilter {
		let filter = GlobFilter::default().with_exclude("*.git*");
		// these are very coarse, we'll need to refine and use other heuristics
		match self {
			CrateQueryScope::PublicApi => {
				filter.with_include("*examples/**/*.rs")
			}
			CrateQueryScope::Internals => filter.with_exclude("*src/**/*.rs"),
		}
	}
}

impl std::fmt::Display for CrateQueryScope {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			CrateQueryScope::PublicApi => write!(f, "usage"),
			CrateQueryScope::Internals => write!(f, "internals"),
		}
	}
}

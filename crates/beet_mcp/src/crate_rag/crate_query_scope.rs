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
			CrateQueryScope::PublicApi => write!(f, "public-api"),
			CrateQueryScope::Internals => write!(f, "internals"),
		}
	}
}


impl TryFrom<&str> for CrateQueryScope {
	type Error = anyhow::Error;

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		match value {
			"public-api" => Ok(CrateQueryScope::PublicApi),
			"internals" => Ok(CrateQueryScope::Internals),
			_ => anyhow::bail!("Invalid CrateQueryScope: {}", value),
		}
	}
}

// #[automatically_derived]
// #[allow(unused_braces)]
// impl schemars::JsonSchema for CrateQueryScope {
// 	/// manual impl to disable references, tools dont like em and we cant serde::flatten enums
// 	fn is_referenceable() -> bool { false }
// 	fn schema_name() -> String { "CrateQueryScope".to_owned() }
// 	fn schema_id() -> Cow<'static, str> {
// 		Cow::Borrowed(std::concat!(
// 			std::module_path!(),
// 			"::",
// 			"CrateQueryScope"
// 		))
// 	}
// 	fn json_schema(generator: &mut SchemaGenerator) -> Schema {
// 		Schema::Object(SchemaObject {
// 			subschemas: Some(Box::new(SubschemaValidation {
// 				one_of: Some(vec![
// 					schemars::_private::metadata::add_description(
// 						schemars::_private::new_unit_enum("PublicApi"),
// 						"How to use the crate, ie examples, tests, documentation. This should be the default scope for most queries.",
// 					),
// 					schemars::_private::metadata::add_description(
// 						schemars::_private::new_unit_enum("Internals"),
// 						"Implementations of engine internals. Only query for this if you are certain you need to know how the internals of a functionas it may misguide you to reimplement engine internals instead of using the public API. an example for an acceptable use is implementing new features for the crate\n\t ",
// 					),
// 				]),
// 				..Default::default()
// 			})),
// 			..Default::default()
// 		})
// 	}
// }

use rmcp::schemars;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;



/// cos enums broken for
#[derive(
	Debug,
	Default,
	Clone,
	Hash,
	PartialEq,
	Eq,
	Serialize,
	Deserialize,
	JsonSchema,
)]
// this sucks but rmcp no enum
#[schemars(description = "\
C
		 ")]
// #[serde(flatten)]
pub struct ContentTypeStr(pub String);


impl Into<ContentType> for ContentTypeStr {
	fn into(self) -> ContentType {
		ContentType::try_from(self.0.as_str()).unwrap_or_default()
	}
}


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
	JsonSchema,
)]
pub enum ContentType {
	#[default]
	#[schemars(description = "\
	How to use the crate, ie examples, tests, documentation. \
	This should be the default scope for most queries.
")]
	Guides,
	#[schemars(description = "\
	How to use the crate, ie examples, tests, documentation. \
	This should be the default scope for most queries.
")]
	Examples,
	#[schemars(description = "\
	Implementations of engine internals. \
	Only query for this if you are certain you need to know how the internals of a function\
	as it may misguide you to reimplement engine internals instead of using the public API. \
	an example for an acceptable use is implementing new features for the crate
		 ")]
	Internals,
}


impl std::fmt::Display for ContentType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ContentType::Guides => write!(f, "guides"),
			ContentType::Examples => write!(f, "examples"),
			ContentType::Internals => write!(f, "internals"),
		}
	}
}


impl TryFrom<&str> for ContentType {
	type Error = anyhow::Error;

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		match value {
			"guides" => Ok(ContentType::Guides),
			"examples" => Ok(ContentType::Examples),
			"internals" => Ok(ContentType::Internals),
			_ => anyhow::bail!("Invalid Content Type: {}", value),
		}
	}
}

impl Into<ContentTypeStr> for ContentType {
	fn into(self) -> ContentTypeStr { ContentTypeStr(self.to_string()) }
}

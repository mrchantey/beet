use rmcp::schemars;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;



#[derive(
	Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
/// this absolutely sucks but schemars/rcmp so broken
pub struct ContentTypeStr {
	#[schemars(description = "\
The type of content to query for. this can **only** be one of
['docs', 'guides', 'examples', 'internals']. 
The reccommended type depends on the query, for example:
- 'how does the related! macro work?' = 'docs'
- 'lets create a simple 3d scene' = 'examples'
- 'what changed in bevy 0.16' = 'guides'
- 'help me add the NonSend attribute to the Component trait' = 'internals'
When in doubt, go with 'examples' as that provides the most holistic usage patterns 
")]
	pub content_type: String,
}

impl Default for ContentTypeStr {
	fn default() -> Self { ContentType::default().into() }
}


impl Into<ContentType> for ContentTypeStr {
	fn into(self) -> ContentType {
		ContentType::try_from(self.content_type.as_str()).unwrap_or_default()
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
	Docs,
	Guides,
	Examples,
	Internals,
}


impl std::fmt::Display for ContentType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ContentType::Docs => write!(f, "docs"),
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
			"docs" => Ok(ContentType::Docs),
			"guides" => Ok(ContentType::Guides),
			"examples" => Ok(ContentType::Examples),
			"internals" => Ok(ContentType::Internals),
			_ => anyhow::bail!("Invalid Content Type: {}", value),
		}
	}
}

impl Into<ContentTypeStr> for ContentType {
	fn into(self) -> ContentTypeStr {
		ContentTypeStr {
			content_type: self.to_string(),
		}
	}
}

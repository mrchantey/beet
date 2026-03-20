use crate::openresponses::Annotation;
use crate::prelude::*;
use beet_core::prelude::*;


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct O11sMeta {
	pub action_id: ActionId,
	pub provider_slug: String,
	pub model_slug: String,
	pub response_id: String,
	pub item_id: String,
	pub content_index: Option<u32>,
	pub call_id: Option<String>,
	pub original_text: Option<String>,
	pub annotations: Vec<Annotation>,
}

impl Document for O11sMeta {
	type Id = ActionId;
	fn id(&self) -> Self::Id { self.action_id }
}
impl O11sMeta {
	pub fn call_id(&self) -> Option<&str> { self.call_id.as_deref() }
}

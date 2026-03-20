use crate::openresponses::Annotation;
use crate::prelude::*;
use beet_core::prelude::*;


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct O11sMeta {
	actor_id: ActorId,
	provider_slug: String,
	response_id: String,
	item_id: String,
	model_name: String,
	content_index: Option<u32>,
	call_id: Option<String>,
	original_text: Option<String>,
	annotations: Vec<Annotation>,
}

impl Document for O11sMeta {
	type Id = (ActorId, String, String, Option<u32>);
	fn id(&self) -> Self::Id { self.id }
}

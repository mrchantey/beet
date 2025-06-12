use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;


/// Constant values used in the HTML rendering process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Resource)]
pub struct HtmlConstants {
	/// the attribute for element ids, used for encoding the [TreePosition],
	pub tree_idx_key: String,
	/// in debug mode, the attribute for the span of the element
	pub span_key: String,
	/// Attrubute tagging the [`TreeLocationMap`](crate::prelude::TreeLocationMap)
	pub loc_map_key: String,
	/// the global event handler for all events
	pub event_handler: String,
	/// the global vec that stores prehydrated events
	pub event_store: String,
	/// Used for setting the style id on elements
	pub style_id_key: String,
	/// When parsing a [`HtmlDocument`], elements with these tags will be hoisted to the head of the document.
	/// Defauts to `["title", "meta", "link", "style", "script", "base"]`.
	pub hoist_to_head_tags: Vec<String>,
}

impl Default for HtmlConstants {
	fn default() -> Self {
		Self {
			tree_idx_key: "data-beet-rsx-idx".to_string(),
			loc_map_key: "data-beet-loc-map".to_string(),
			span_key: "data-beet-span".to_string(),
			event_handler: "_beet_event_handler".to_string(),
			event_store: "_beet_event_store".to_string(),
			style_id_key: "data-beet-style-id".to_string(),
			hoist_to_head_tags: vec![
				"title".to_string(),
				"meta".to_string(),
				"link".to_string(),
				"style".to_string(),
				"script".to_string(),
				"base".to_string(),
			],
		}
	}
}
impl HtmlConstants {
	/// Returns the attribute key for the style id
	pub fn style_id_class(&self, id: u64) -> String {
		format!("{}-{}", self.style_id_key, id)
	}
}

use std::path::PathBuf;

use bevy::prelude::*;

use crate::node::StyleId;

/// Constant values used in the HTML rendering process.
#[derive(Debug, Clone, PartialEq, Resource)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HtmlConstants {
	/// the attribute for element ids, used for encoding the [TreePosition],
	pub dom_idx_key: String,
	/// in debug mode, the attribute for the span of the element
	pub span_key: String,
	/// the global event handler for all events
	pub event_handler: String,
	/// the global vec that stores prehydrated events
	pub event_store: String,
	/// Used for setting the style id on elements
	pub style_id_key: String,
	/// The client island scene is stored in a script tag with this type
	pub client_islands_script_type: String,
	/// Path to the wasm directory, defaults to `wasm`
	pub wasm_dir: PathBuf,
	/// Name of the wasm js and bin files, defaults to `main`
	pub wasm_name: String,
	/// When parsing a [`HtmlDocument`], elements with these tags will be hoisted to the head of the document.
	/// Defauts to `["title", "meta", "link", "style", "script", "base"]`.
	pub hoist_to_head_tags: Vec<String>,
}

impl Default for HtmlConstants {
	fn default() -> Self {
		Self {
			dom_idx_key: "data-beet-dom-idx".into(),
			span_key: "data-beet-span".into(),
			event_handler: "_beet_event_handler".into(),
			event_store: "_beet_event_store".into(),
			style_id_key: "data-beet-style-id".into(),
			client_islands_script_type: "beet/client-islands".into(),
			wasm_dir: "wasm".into(),
			wasm_name: "main".into(),
			hoist_to_head_tags: vec![
				"title".into(),
				"meta".into(),
				"link".into(),
				"style".into(),
				"script".into(),
				"base".into(),
			],
		}
	}
}
impl HtmlConstants {
	/// Returns the attribute key for the style id
	pub fn style_id_attribute(&self, id: StyleId) -> String {
		format!("{}-{}", self.style_id_key, *id)
	}

	pub fn wasm_bin_url(&self) -> String {
		format!(
			"/{}/{}_bg.wasm",
			self.wasm_dir.to_string_lossy(),
			self.wasm_name
		)
	}
	pub fn wasm_js_url(&self) -> String {
		format!("/{}/{}.js", self.wasm_dir.to_string_lossy(), self.wasm_name)
	}
}

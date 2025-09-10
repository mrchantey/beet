use bevy::prelude::*;
use std::path::PathBuf;

/// Constant values used in the HTML rendering process.
#[derive(Debug, Clone, PartialEq, Resource, Reflect)]
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
	/// The <script type="x"/> for the client island scene
	pub client_islands_script_type: String,
	/// The bt in <!--bt|32-->dynamic content<!--/bt--> for text nodes
	/// This must *not* contain a `|` pipe as that is used to split the id
	pub text_node_marker: String,
	/// Path to the wasm directory, defaults to `wasm`
	pub wasm_dir: PathBuf,
	/// Name of the wasm js and bin files, defaults to `main`
	pub wasm_name: String,
	/// Tags whose inner text content is 'escaped', ie not parsed as rsx
	pub raw_text_elements: std::collections::HashSet<&'static str>,
	/// Tags that should not have style ids applied to them
	pub ignore_style_id_tags: Vec<String>,
	/// When parsing a [`HtmlDocument`], elements with these tags will be hoisted to the head of the document.
	/// Defauts to `["title", "meta", "link", "style", "script", "base"]`.
	pub hoist_to_head_tags: Vec<String>,
	/// This type is used by rstml to determine if an element should be treated as self-closing.
	pub self_closing_elements: std::collections::HashSet<&'static str>,
}

impl Default for HtmlConstants {
	fn default() -> Self {
		let hoist_to_head_tags = vec![
			"title".into(),
			"meta".into(),
			"link".into(),
			"style".into(),
			"script".into(),
			"base".into(),
		];

		Self {
			dom_idx_key: "data-beet-dom-idx".into(),
			span_key: "data-beet-span".into(),
			event_handler: "_beet_event_handler".into(),
			event_store: "_beet_event_store".into(),
			style_id_key: "data-beet-style-id".into(),
			client_islands_script_type: "beet/client-islands".into(),
			text_node_marker: "bt".into(),
			wasm_dir: "wasm".into(),
			wasm_name: "main".into(),
			raw_text_elements: ["script", "style", "code"]
				// raw_text_elements: ["script", "style"]
				.into_iter()
				.collect(),
			ignore_style_id_tags: hoist_to_head_tags
				.iter()
				.cloned()
				.chain(["html".into(), "head".into()])
				.collect(),
			hoist_to_head_tags,
			self_closing_elements: [
				"area", "base", "br", "col", "embed", "hr", "img", "input",
				"link", "meta", "param", "source", "track", "wbr",
			]
			.into_iter()
			.collect(),
		}
	}
}
impl HtmlConstants {
	/// Returns the attribute key for the style id
	pub fn style_id_attribute(&self, id: u64) -> String {
		format!("{}-{}", self.style_id_key, id)
	}
	/// Added by the [parse_lightning] step, and replaced by the apply_style_id step
	pub fn style_id_attribute_placeholder(&self) -> String {
		format!("{}-PLACEHOLDER", self.style_id_key)
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

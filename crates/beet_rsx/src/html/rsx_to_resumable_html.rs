use crate::prelude::*;
use anyhow::Result;

#[derive(Default)]
pub struct RsxToResumableHtmlString {
	pub rsx_to_resumable: RsxToResumableHtml,
	pub render_html: RenderHtml,
}

impl RsxPlugin<RsxRoot, String> for RsxToResumableHtmlString {
	fn apply(self, root: RsxRoot) -> Result<String> {
		Ok(root
			.pipe(self.rsx_to_resumable)?
			.pipe1(self.render_html)?
			.take1())
	}
}

#[derive(Default)]
pub struct RsxToResumableHtml {
	pub rsx_to_html: RsxToHtml,
	pub html_to_document: HtmlToDocument,
	pub html_doc_to_resumable: HtmlDocToResumable,
}

impl<T: AsRef<RsxNode> + RsxPluginTarget> RsxPlugin<T, (T, HtmlDocument)>
	for RsxToResumableHtml
where
	(T, HtmlDocument): RsxPluginTarget,
{
	fn apply(self, root: T) -> Result<(T, HtmlDocument)> {
		let html = root
			.as_ref()
			.pipe(self.rsx_to_html)?
			.pipe1(self.html_to_document)?
			.pipe(self.html_doc_to_resumable)?
			.take1();
		Ok((root, html))
	}
}


#[derive(Default)]
pub struct HtmlDocToResumable {
	pub html_constants: HtmlConstants,
}

impl<T: AsRef<RsxNode> + RsxPluginTarget>
	RsxPlugin<(T, HtmlDocument), (T, HtmlDocument)> for HtmlDocToResumable
where
	(T, HtmlDocument): RsxPluginTarget,
{
	fn apply(
		self,
		(node, mut doc): (T, HtmlDocument),
	) -> Result<(T, HtmlDocument)> {
		self.insert_tree_location_map(node.as_ref(), &mut doc);
		self.insert_catch_prehydrated_events(&mut doc);
		self.insert_wasm_script(&mut doc);
		Ok((node, doc))
	}
}

impl HtmlDocToResumable {
	/// attempt to insert the rsx context map into the html body,
	/// otherwise append it to the end of the html
	fn insert_tree_location_map(&self, node: &RsxNode, doc: &mut HtmlDocument) {
		let loc_map = TreeLocationMap::from_node(node).to_csv();
		let el = HtmlElementNode::inline_script(loc_map, vec![HtmlAttribute {
			key: self.html_constants.loc_map_key.to_string(),
			value: None,
		}]);
		doc.body.push(el.into());
	}

	fn insert_catch_prehydrated_events(&self, doc: &mut HtmlDocument) {
		let script = format!(
			r#"
// console.log('sweet has loaded')
globalThis.{prehydrate_events} = []
globalThis.{event_handler} = (id,event) => globalThis.{prehydrate_events}.push([id, event])
"#,
			prehydrate_events = self.html_constants.event_store,
			event_handler = self.html_constants.event_handler,
		);
		let el = HtmlElementNode::inline_script(script, vec![HtmlAttribute {
			key: "type".to_string(),
			value: Some("module".to_string()),
		}]);
		doc.body.push(el.into());
	}

	fn insert_wasm_script(&self, doc: &mut HtmlDocument) {
		let script = r#"
		import init from './wasm/bindgen.js'
		init('./wasm/bindgen_bg.wasm')
			.catch((error) => {
				if (!error.message.startsWith("Using exceptions for control flow,"))
					throw error
			})
"#;
		doc.body.push(HtmlNode::Element(HtmlElementNode {
			tag: "script".to_string(),
			self_closing: false,
			attributes: vec![HtmlAttribute {
				key: "type".to_string(),
				value: Some("module".to_string()),
			}],
			children: vec![HtmlNode::Text(script.to_string())],
		}));
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;




	#[test]
	fn plain() {
		expect(
			rsx! { <br /> }
				.pipe(RsxToResumableHtmlString::default())
				.unwrap(),
		)
		.to_contain("<br/>");
	}
	#[test]
	fn id() {
		expect(
			rsx! {
				<main>
					<article>{7}</article>
				</main>
			}
			.pipe(RsxToResumableHtmlString::default())
			.unwrap(),
		)
		.to_contain(
			"<main><article data-beet-rsx-idx=\"1\">7</article></main>",
		);
	}
	#[test]
	fn events() {
		let on_click = |_| {};

		expect(
			rsx! { <main onclick=on_click></main> }
			.pipe(RsxToResumableHtmlString::default())
			.unwrap(),
		)
		.to_contain("<main onclick=\"_beet_event_handler(0, event)\" data-beet-rsx-idx=\"0\"></main>");
	}
}

use crate::prelude::*;

/// insert tags and info required for client side resumability
#[derive(Default)]
pub struct HtmlDocToResumable {
	pub html_constants: HtmlConstants,
}

impl<T: AsRef<RsxNode> + RsxPipelineTarget>
	RsxPipeline<(HtmlDocument, T), HtmlDocument> for HtmlDocToResumable
where
	(HtmlDocument, T): RsxPipelineTarget,
{
	fn apply(self, (mut doc, node): (HtmlDocument, T)) -> HtmlDocument {
		self.insert_tree_location_map(node.as_ref(), &mut doc);
		self.insert_catch_prehydrated_events(&mut doc);
		self.insert_wasm_script(&mut doc);
		doc
	}
}

impl HtmlDocToResumable {
	/// attempt to insert the rsx context map into the html body,
	/// otherwise append it to the end of the html
	fn insert_tree_location_map(&self, node: &RsxNode, doc: &mut HtmlDocument) {
		let loc_map = node.pipe(NodeToTreeLocationMap);
		let loc_map =
			ron::ser::to_string_pretty(&loc_map, Default::default()).unwrap();
		let el = HtmlElementNode::inline_script(loc_map, vec![
			HtmlAttribute {
				key: "type".to_string(),
				value: Some("beet/ron".to_string()),
			},
			HtmlAttribute {
				key: self.html_constants.loc_map_key.to_string(),
				value: None,
			},
		]);
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
		doc.body.push(
			HtmlElementNode::inline_script(script, vec![HtmlAttribute {
				key: "type".to_string(),
				value: Some("module".to_string()),
			}])
			.into(),
		);
	}

	fn insert_wasm_script(&self, doc: &mut HtmlDocument) {
		let script = r#"
		import init from '/wasm/bindgen.js'
		init('/wasm/bindgen_bg.wasm')
			.catch((error) => {
				if (!error.message.startsWith("Using exceptions for control flow,"))
					throw error
			})
"#;
		doc.body.push(
			HtmlElementNode::inline_script(script.to_string(), vec![
				HtmlAttribute {
					key: "type".to_string(),
					value: Some("module".to_string()),
				},
			])
			.into(),
		);
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	fn pipe(root: RsxRoot) -> String {
		root.as_ref()
			.pipe(RsxToHtml::default())
			.pipe(HtmlToDocument::default())
			.unwrap()
			.map(|doc| (doc, root.as_ref()))
			.pipe(HtmlDocToResumable::default())
			.pipe(RenderHtml::default())
			.unwrap()
	}


	#[test]
	fn plain() { expect(pipe(rsx! { <br /> })).to_contain("<br/>"); }
	#[test]
	fn id() {
		expect(pipe(rsx! {
			<main>
				<article>{7}</article>
			</main>
		}))
		.to_contain(
			"<main><article data-beet-rsx-idx=\"2\">7</article></main>",
		);
	}
	#[test]
	fn events() {
		let on_click = |_| {};

		expect(
			pipe(rsx! { <main onclick=on_click></main> })
		)
		.to_contain("<main onclick=\"_beet_event_handler(1, event)\" data-beet-rsx-idx=\"1\"></main>");
	}
}

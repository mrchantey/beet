use crate::prelude::*;





#[derive(Default)]
pub struct RsxToResumableHtml {
	/// add attributes required for resumability
	pub html_constants: HtmlConstants,
}
impl RsxToResumableHtml {
	pub fn render_body(node: impl AsRef<RsxNode>) -> String {
		Self::default().map_node(node).render()
	}

	pub fn map_node(&mut self, node: impl AsRef<RsxNode>) -> HtmlDocument {
		let node = node.as_ref();
		let mut html = RsxToHtml::default().map_node(node).into_document();
		self.insert_tree_location_map(node, &mut html);
		self.insert_catch_prehydrated_events(&mut html);
		html
	}

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
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn plain() {
		expect(RsxToResumableHtml::render_body(&rsx! { <br /> }))
			.to_contain("<br/>");
	}
	#[test]
	fn id() {
		expect(RsxToResumableHtml::render_body(&rsx! {
			<main>
				<article>{7}</article>
			</main>
		}))
		.to_contain(
			"<main><article data-beet-rsx-idx=\"1\">7</article></main>",
		);
	}
	#[test]
	fn events() {
		let on_click = |_| {};

		expect(RsxToResumableHtml::render_body(
			&rsx! { <main onclick=on_click></main> },
		))
		.to_contain("<main onclick=\"_beet_event_handler(0, event)\" data-beet-rsx-idx=\"0\"></main>");
	}
}

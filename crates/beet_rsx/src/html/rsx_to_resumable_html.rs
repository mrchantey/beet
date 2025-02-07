use crate::prelude::*;





#[derive(Default)]
pub struct RsxToResumableHtml {
	/// add attributes required for resumability
	pub html_constants: HtmlConstants,
	/// tracking this allows us to match with [DomLocation]
	/// TODO this is terrifying we should somehow use [DomLocationVisitor]?
	pub dom_idx_incr: usize,
}
impl RsxToResumableHtml {
	pub fn render_body(node: impl AsRef<RsxNode>) -> String {
		Self::default().map_node(node).render()
	}

	pub fn map_node(&mut self, node: impl AsRef<RsxNode>) -> HtmlDocument {
		let node = node.as_ref();
		let mut html = RsxToHtml::as_resumable().map_node(node).into_document();
		for node in html.iter_mut() {
			self.visit_node(node);
		}
		self.insert_dom_location_map(node, &mut html);
		self.insert_catch_prehydrated_events(&mut html);
		html
	}

	fn visit_node(&mut self, node: &mut HtmlNode) {
		// TODO to feature parity with [DomLocation] we should
		// increment after every single node visited, not just elements
		self.dom_idx_incr += 1;
		match node {
			HtmlNode::Element(el) => {
				let actual_this_sucks_dom_idx_incr = self.dom_idx_incr - 1;
				for attr in el.attributes.iter_mut() {
					if attr.key == "needs-id" {
						attr.key = self.html_constants.rsx_idx_key.to_string();
						attr.value =
							Some(actual_this_sucks_dom_idx_incr.to_string());
					} else if attr.key.starts_with("on") {
						attr.value = Some(format!(
							"{}({}, event)",
							self.html_constants.event_handler,
							actual_this_sucks_dom_idx_incr,
						));
					}
				}



				for child in &mut el.children {
					self.visit_node(child);
				}
			}
			_ => {}
		}
	}

	/// attempt to insert the rsx context map into the html body,
	/// otherwise append it to the end of the html
	fn insert_dom_location_map(&self, node: &RsxNode, doc: &mut HtmlDocument) {
		let loc_map = DomLocationMap::from_node(node).to_csv();
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
	use crate::prelude::*;
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

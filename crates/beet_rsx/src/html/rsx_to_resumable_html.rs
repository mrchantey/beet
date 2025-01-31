use crate::prelude::*;





#[derive(Default)]
pub struct RsxToResumableHtml {
	/// add attributes required for resumability
	pub html_constants: HtmlConstants,
	/// tracking this allows us to match with [RsxContext]
	pub num_html_elements: usize,
}
impl RsxToResumableHtml {
	pub fn render_body(node: &RsxNode) -> String {
		Self::default().map_node(node).render()
	}

	pub fn map_node(&mut self, node: &RsxNode) -> HtmlDocument {
		let mut html = RsxToHtml::as_resumable().map_node(node).into_document();
		for node in html.iter_mut() {
			self.visit_node(node);
		}
		self.insert_rsx_context_map(node, &mut html);
		self.insert_catch_prehydrated_events(&mut html);
		html
	}

	fn visit_node(&mut self, node: &mut HtmlNode) {
		match node {
			HtmlNode::Element(el) => {
				for attr in el.attributes.iter_mut() {
					if attr.key == "needs-id" {
						attr.key = self.html_constants.id_key.to_string();
						attr.value = Some(self.num_html_elements.to_string());
					} else if attr.key.starts_with("on") {
						attr.value = Some(format!(
							"{}({}, event)",
							self.html_constants.event_handler,
							self.num_html_elements,
						));
					}
				}

				self.num_html_elements += 1;


				for child in &mut el.children {
					self.visit_node(child);
				}
			}
			_ => {}
		}
	}

	/// attempt to insert the rsx context map into the html body,
	/// otherwise append it to the end of the html
	fn insert_rsx_context_map(&self, node: &RsxNode, doc: &mut HtmlDocument) {
		let rsx_context_map = RsxContextMap::from_node(node).to_csv();
		let el = HtmlElementNode::inline_script(rsx_context_map, vec![
			HtmlAttribute {
				key: self.html_constants.cx_map_key.to_string(),
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
		expect(RsxToResumableHtml::render_body(&rsx! { <br/> }))
			.to_contain("<br/>");
	}
	#[test]
	fn id() {
		expect(RsxToResumableHtml::render_body(
			&rsx! { <main><article>{7}</article></main> },
		))
		.to_contain("<main><article data-sweet-id=\"1\">7</article></main>");
	}
	#[test]
	fn events() {
		let on_click = |_| {};

		expect(RsxToResumableHtml::render_body(
			&rsx! { <main onclick=on_click></main> },
		))
		.to_contain("<main onclick=\"_sweet_event_handler(0, event)\" data-sweet-id=\"0\"></main>");
	}
}

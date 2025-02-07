use crate::prelude::*;





///
pub struct RsxToHtml {
	/// on elements that directly contain rust code (non recursive),
	/// give them a `needs-id` attribute to be mapped by [RsxToResumableHtml]
	pub mark_needs_id: bool,
	/// text node content will be trimmed
	pub trim: bool,
}


impl Default for RsxToHtml {
	fn default() -> Self {
		Self {
			mark_needs_id: false,
			trim: false,
		}
	}
}


impl RsxToHtml {
	pub fn as_resumable() -> Self {
		Self {
			mark_needs_id: true,
			trim: false,
		}
	}

	/// convenience so you dont have to add
	/// a `.render()` at the end of a long rsx macro
	pub fn render_body(node: impl AsRef<RsxNode>) -> String {
		Self::default().map_node(node).render()
	}

	pub fn map_node(&mut self, rsx_node: impl AsRef<RsxNode>) -> Vec<HtmlNode> {
		match rsx_node.as_ref() {
			RsxNode::Fragment(nodes) => {
				nodes.iter().map(|n| self.map_node(n)).flatten().collect()
			}
			RsxNode::Component(RsxComponent { node, .. }) => {
				self.map_node(node)
			}
			RsxNode::Block(RsxBlock { initial, .. }) => self.map_node(initial),
			RsxNode::Element(e) => {
				vec![HtmlNode::Element(self.map_element(e))]
			}
			RsxNode::Text(str) => {
				let str = if self.trim { str.trim() } else { str };
				vec![HtmlNode::Text(str.into())]
			}
			RsxNode::Comment(str) => {
				vec![HtmlNode::Comment(str.clone())]
			}
			RsxNode::Doctype => {
				vec![HtmlNode::Doctype]
			}
		}
	}

	pub fn map_element(&mut self, rsx_el: &RsxElement) -> HtmlElementNode {
		let mut html_attributes = rsx_el
			.attributes
			.iter()
			.map(|a| self.map_attribute(a))
			.flatten()
			.collect::<Vec<_>>();

		if self.mark_needs_id && rsx_el.contains_rust() {
			html_attributes.push(HtmlAttribute {
				key: "needs-id".to_string(),
				value: None,
			});
		}

		HtmlElementNode {
			tag: rsx_el.tag.clone(),
			self_closing: rsx_el.self_closing,
			attributes: html_attributes,
			children: rsx_el
				.children
				.iter()
				.map(|c| self.map_node(c))
				.flatten()
				.collect(),
		}
	}

	pub fn map_attribute(&self, attr: &RsxAttribute) -> Vec<HtmlAttribute> {
		match attr {
			RsxAttribute::Key { key } => vec![HtmlAttribute {
				key: key.clone(),
				value: None,
			}],
			RsxAttribute::KeyValue { key, value } => vec![HtmlAttribute {
				key: key.clone(),
				value: Some(value.clone()),
			}],
			RsxAttribute::BlockValue { key, initial, .. } => {
				vec![HtmlAttribute {
					key: key.clone(),
					value: Some(initial.clone()),
				}]
			}
			RsxAttribute::Block { initial, .. } => initial
				.iter()
				.map(|a| self.map_attribute(a))
				.flatten()
				.collect(),
		}
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn doctype() {
		// HtmlRenderer::render_body_default
		expect(RsxToHtml::render_body(&rsx! { <!DOCTYPE html> }))
			.to_be("<!DOCTYPE html>");
	}

	#[test]
	fn comment() {
		expect(RsxToHtml::render_body(&rsx! { <!-- "hello" --> }))
			.to_be("<!-- hello -->");
	}

	#[test]
	fn text() {
		expect(RsxToHtml::render_body(&rsx! { "hello" })).to_be("hello");
	}

	#[test]
	fn element() {
		let _key = "hidden";
		let _key_value = "class=\"pretty\"";
		let food = "pizza";
		expect(RsxToHtml::render_body(&rsx! {
			<div
				name="pete"
				age=9
				// {key}
				// {key_value}
				favorite_food=food
			></div>
		}))
		.to_be("<div name=\"pete\" age=\"9\" favorite_food=\"pizza\"></div>");
	}
	#[test]
	fn element_self_closing() {
		expect(RsxToHtml::render_body(&rsx! { <br /> })).to_be("<br/>");
	}
	#[test]
	fn element_children() {
		expect(RsxToHtml::render_body(&rsx! { <div>hello</div> }))
			.to_be("<div>hello</div>");
	}

	#[test]
	fn rsx_text() {
		let value = "hello";
		expect(RsxToHtml::render_body(&rsx! { {value} })).to_be("hello");
	}

	#[test]
	fn nested() {
		let world = "mars";
		expect(RsxToHtml::render_body(&rsx! {
			<div>
				<p>hello {world}</p>
			</div>
		}))
		.to_be("<div><p>hello mars</p></div>");
	}
	#[test]
	fn events() {
		let onclick = |_| {};
		let world = "mars";
		expect(RsxToHtml::render_body(&rsx! {
			<div onclick=onclick>
				<p>hello {world}</p>
			</div>
		}))
		.to_be("<div onclick=\"event-placeholder\"><p>hello mars</p></div>");
	}

	#[test]
	fn component_props() {
		struct Child {
			value: usize,
		}
		impl Component for Child {
			fn render(self) -> RsxRoot {
				rsx! { <p>hello {self.value}</p> }
			}
		}
		let node = rsx! { <div>the child is <Child value=38 />!</div> };

		expect(RsxToHtml::render_body(&node))
			.to_be("<div>the child is <p>hello 38</p>!</div>");
	}
	#[test]
	fn component_children() {
		struct Layout;
		impl Component for Layout {
			fn render(self) -> RsxRoot {
				rsx! {
					<div>
						<h1>welcome</h1>
						<p>
							<slot />
						</p>
					</div>
				}
			}
		}
		let node = rsx! {
			<Layout>
				<b>foo</b>
			</Layout>
		};

		expect(RsxToHtml::render_body(&node))
			.to_be("<div><h1>welcome</h1><p><b>foo</b></p></div>");
	}
	#[test]
	fn component_slots() {
		struct Layout;
		impl Component for Layout {
			fn render(self) -> RsxRoot {
				rsx! {
					<article>
						<h1>welcome</h1>
						<p>
							<slot name="tagline" />
						</p>
						<main>
							<slot />
						</main>
					</article>
				}
			}
		}
		let node = rsx! {
			<Layout>
				<b slot="tagline">what a cool article</b>
				<div>direct child</div>
			</Layout>
		};

		expect(RsxToHtml::render_body(&node))
			.to_be("<article><h1>welcome</h1><p><b>what a cool article</b></p><main><div>direct child</div></main></article>");
	}


	#[test]
	fn trims() {
		expect(
			RsxToHtml {
				trim: true,
				..Default::default()
			}
			.map_node(&rsx! { "  hello  " })
			.render(),
		)
		.to_be("hello");
	}
}

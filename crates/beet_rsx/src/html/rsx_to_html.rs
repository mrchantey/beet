use crate::prelude::*;


#[derive(Debug, Default)]
pub struct RsxToHtml {
	/// add attributes required for resumability
	pub html_constants: HtmlConstants,
	/// on elements that directly contain rust code (non recursive),
	/// give them a `needs-id` attribute to be mapped by [RsxToResumableHtml]
	pub no_beet_attributes: bool,
	/// text node content will be trimmed
	pub trim: bool,
	tree_idx_incr: TreeIdxIncr,
}


impl RsxToHtml {
	pub fn as_resumable() -> Self {
		Self {
			no_beet_attributes: false,
			..Default::default()
		}
	}

	/// convenience so you dont have to add
	/// a `.render()` at the end of a long rsx macro
	pub fn render_body(root: &RsxRoot) -> String {
		Self::default().map_root(root).render()
	}

	pub fn map_root(&mut self, root: &RsxRoot) -> Vec<HtmlNode> {
		// do we need to use location?
		self.map_node(&root.node)
	}

	/// recursively map rsx nodes to html nodes
	/// ## Panics
	/// If slot children have not been applied
	pub fn map_node(&mut self, node: impl AsRef<RsxNode>) -> Vec<HtmlNode> {
		let idx = self.tree_idx_incr.next();
		match node.as_ref() {
			RsxNode::Doctype { .. } => vec![HtmlNode::Doctype],
			RsxNode::Comment { value, .. } => {
				vec![HtmlNode::Comment(value.clone())]
			}
			RsxNode::Text { value, .. } => {
				let str = if self.trim { value.trim() } else { value };
				vec![HtmlNode::Text(str.into())]
			}
			RsxNode::Element(e) => {
				vec![HtmlNode::Element(self.map_element(idx, e))]
			}
			RsxNode::Fragment { nodes, .. } => {
				nodes.iter().map(|n| self.map_node(n)).flatten().collect()
			}
			RsxNode::Block(rsx_block) => self.map_node(&rsx_block.initial.node),
			RsxNode::Component(RsxComponent {
				root,
				slot_children,
				..
			}) => {
				slot_children.assert_empty();
				// use the location of the root
				self.map_node(&root.node)
			}
		}
	}

	pub fn map_element(
		&mut self,
		idx: TreeIdx,
		el: &RsxElement,
	) -> HtmlElementNode {
		let mut html_attributes = el
			.attributes
			.iter()
			.map(|a| self.map_attribute(idx, a))
			.flatten()
			.collect::<Vec<_>>();

		if !self.no_beet_attributes && el.contains_rust() {
			html_attributes.push(HtmlAttribute {
				key: self.html_constants.tree_idx_key.to_string(),
				value: Some(idx.to_string()),
			});
		}

		HtmlElementNode {
			tag: el.tag.clone(),
			self_closing: el.self_closing,
			attributes: html_attributes,
			children: self.map_node(&el.children),
		}
	}

	pub fn map_attribute(
		&self,
		idx: TreeIdx,
		attr: &RsxAttribute,
	) -> Vec<HtmlAttribute> {
		match attr {
			RsxAttribute::Key { key } => vec![HtmlAttribute {
				key: key.clone(),
				value: None,
			}],
			RsxAttribute::KeyValue { key, value } => {
				vec![HtmlAttribute {
					key: key.clone(),
					value: Some(value.clone()),
				}]
			}
			RsxAttribute::BlockValue { key, initial, .. } => {
				if !self.no_beet_attributes && key.starts_with("on") {
					vec![HtmlAttribute {
						key: key.clone(),
						value: Some(format!(
							"{}({}, event)",
							self.html_constants.event_handler,
							idx.to_string(),
						)),
					}]
				} else {
					vec![HtmlAttribute {
						key: key.clone(),
						value: Some(initial.clone()),
					}]
				}
			}
			RsxAttribute::Block { initial, .. } => initial
				.iter()
				.map(|a| self.map_attribute(idx, a))
				.flatten()
				.collect(),
		}
	}
}




#[cfg(test)]
mod test {
	use crate::as_beet::*;
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
		.to_be("<div name=\"pete\" age=\"9\" favorite_food=\"pizza\" data-beet-rsx-idx=\"0\"></div>");
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
		.to_be("<div><p data-beet-rsx-idx=\"1\">hello mars</p></div>");
	}
	#[test]
	fn events() {
		let onclick = move |_| {};
		let world = "mars";
		expect(RsxToHtml::render_body(&rsx! {
			<div onclick=onclick>
				<p>hello {world}</p>
			</div>
		}))
		.to_be("<div onclick=\"_beet_event_handler(0, event)\" data-beet-rsx-idx=\"0\"><p data-beet-rsx-idx=\"1\">hello mars</p></div>");
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

		expect(RsxToHtml::render_body(&node)).to_be(
			"<div>the child is <p data-beet-rsx-idx=\"4\">hello 38</p>!</div>",
		);
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
		expect(
			rsx! {
				<Layout>
					<b>foo</b>
				</Layout>
			}
			.apply_and_render(),
		)
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

		expect(rsx! {
			<Layout>
				<b slot="tagline">what a cool article</b>
				<div>direct child</div>
			</Layout>
		}.apply_and_render())
			.to_be("<article><h1>welcome</h1><p><b>what a cool article</b></p><main><div>direct child</div></main></article>");
	}


	#[test]
	fn trims() {
		expect(
			RsxToHtml {
				trim: true,
				..Default::default()
			}
			.map_root(&rsx! { "  hello  " })
			.render(),
		)
		.to_be("hello");
	}
}

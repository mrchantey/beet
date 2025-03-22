use crate::prelude::*;

#[derive(Debug, Default)]
pub struct RsxToHtml {
	/// add attributes required for resumability
	pub html_constants: HtmlConstants,
	/// Do not check for wasm, which disables
	/// applying the [`HtmlConstants::tree_idx_key`] on elements that directly contain rust code (non recursive)
	/// 2. applying resumability to trees with `client:` directives.
	pub no_wasm: bool,
	/// text node content will be trimmed
	pub trim: bool,
	tree_idx_incr: TreeIdxIncr,
}

impl<T: RsxPipelineTarget + AsRef<RsxNode>> RsxPipeline<T, Vec<HtmlNode>>
	for RsxToHtml
{
	fn apply(mut self, node: T) -> Vec<HtmlNode> {
		self.map_node(node.as_ref())
	}
}



impl RsxToHtml {
	pub fn as_resumable() -> Self {
		Self {
			no_wasm: false,
			..Default::default()
		}
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

		if !self.no_wasm && el.contains_rust() {
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
				if !self.no_wasm && key.starts_with("on") {
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
		expect(
			rsx! { <!DOCTYPE html> }
				.pipe(RsxToHtmlString::default())
				.unwrap(),
		)
		.to_be("<!DOCTYPE html>");
	}

	#[test]
	fn comment() {
		expect(
			rsx! { <!-- "hello" --> }
				.pipe(RsxToHtmlString::default())
				.unwrap(),
		)
		.to_be("<!-- hello -->");
	}

	#[test]
	fn text() {
		expect(rsx! { "hello" }.pipe(RsxToHtmlString::default()).unwrap())
			.to_be("hello");
	}

	#[test]
	fn element() {
		let _key = "hidden";
		let _key_value = "class=\"pretty\"";
		let food = "pizza";
		expect(rsx! {
			<div
				name="pete"
				age=9
				// {key}
				// {key_value}
				favorite_food=food
			></div>
		}.pipe(RsxToHtmlString::default()).unwrap())
		.to_be("<div name=\"pete\" age=\"9\" favorite_food=\"pizza\" data-beet-rsx-idx=\"0\"></div>");
	}
	#[test]
	fn element_self_closing() {
		expect(rsx! { <br /> }.pipe(RsxToHtmlString::default()).unwrap())
			.to_be("<br/>");
	}
	#[test]
	fn element_children() {
		expect(
			rsx! { <div>hello</div> }
				.pipe(RsxToHtmlString::default())
				.unwrap(),
		)
		.to_be("<div>hello</div>");
	}

	#[test]
	fn rsx_text() {
		let value = "hello";
		expect(rsx! { {value} }.pipe(RsxToHtmlString::default()).unwrap())
			.to_be("hello");
	}

	#[test]
	fn nested() {
		let world = "mars";
		expect(
			rsx! {
				<div>
					<p>hello {world}</p>
				</div>
			}
			.pipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be("<div><p data-beet-rsx-idx=\"1\">hello mars</p></div>");
	}
	#[test]
	fn events() {
		let onclick = move |_| {};
		let world = "mars";
		expect(rsx! {
			<div onclick=onclick>
				<p>hello {world}</p>
			</div>
		}.pipe(RsxToHtmlString::default()).unwrap())
		.to_be("<div onclick=\"_beet_event_handler(0, event)\" data-beet-rsx-idx=\"0\"><p data-beet-rsx-idx=\"1\">hello mars</p></div>");
	}

	#[test]
	fn component() {
		#[derive(Node)]
		struct Child;
		fn child(_: Child) -> RsxRoot {
			rsx! { <p>hello {1}</p> }
		}
		expect(
			rsx! { <Child/> }
				.pipe(RsxToHtmlString::default())
				.unwrap(),
		)
		.to_be(
			// the component itsself is rsx-idx-0
			"<p data-beet-rsx-idx=\"1\">hello 1</p>",
		);

	}
		#[test]
	fn component_props() {
		#[derive(Node)]
		struct Child {
			value: usize,
		}
		fn child(props: Child) -> RsxRoot {
			rsx! { <p>hello {props.value}</p> }
		}

		expect(
			rsx! { <div>the child is <Child value=38 />!</div> }
				.pipe(RsxToHtmlString::default())
				.unwrap(),
		)
		.to_be(
			"<div>the child is <p data-beet-rsx-idx=\"4\">hello 38</p>!</div>",
		);
	}
	#[test]
	fn component_children() {
		#[derive(Node)]
		struct Layout;
		fn layout(_: Layout) -> RsxRoot {
			rsx! {
				<div>
					<h1>welcome</h1>
					<p>
						<slot />
					</p>
				</div>
			}
		}
		expect(
			rsx! {
				<Layout>
					<b>foo</b>
				</Layout>
			}
			.pipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be("<div><h1>welcome</h1><p><b>foo</b></p></div>");
	}
	#[test]
	fn component_slots() {
		#[derive(Node)]
		struct Layout;
		fn layout(_: Layout) -> RsxRoot {
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

		expect(rsx! {
			<Layout>
				<b slot="tagline">what a cool article</b>
				<div>direct child</div>
			</Layout>
		}.pipe(RsxToHtmlString::default()).unwrap())
			.to_be("<article><h1>welcome</h1><p><b>what a cool article</b></p><main><div>direct child</div></main></article>");
	}


	#[test]
	fn trims() {
		expect(
			rsx! { "  hello  " }
				.pipe(RsxToHtmlString {
					rsx_to_html: RsxToHtml {
						trim: true,
						..Default::default()
					},
					..Default::default()
				})
				.unwrap(),
		)
		.to_be("hello");
	}
}

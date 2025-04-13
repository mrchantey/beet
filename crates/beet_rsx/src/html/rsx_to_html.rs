use crate::prelude::*;



/// Convert [`RsxNode`] structures into a string of [`HtmlNode`]
///
/// ## Panics
/// - Panics if `no_slot_check` is false and there are still slot children
/// - Panics if an element is self closing and has children
#[derive(Debug, Default)]
pub struct RsxToHtml {
	/// Slot children are not rendered so by default this will
	/// panic if they still exist, enable this option to ignore
	/// slot children, pretty much only used for testing
	pub no_slot_check: bool,
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

impl<T: AsRef<RsxNode>> Pipeline<T, Vec<HtmlNode>> for RsxToHtml {
	fn apply(mut self, node: T) -> Vec<HtmlNode> { self.map_node(node) }
}



impl RsxToHtml {
	pub fn no_slot_check(mut self) -> Self {
		self.no_slot_check = true;
		self
	}

	/// recursively map rsx nodes to html nodes
	/// ## Panics
	/// If slot children have not been applied
	pub fn map_node(&mut self, node: impl AsRef<RsxNode>) -> Vec<HtmlNode> {
		let idx = self.tree_idx_incr.next();

		match node.as_ref() {
			RsxNode::Doctype(_) => vec![HtmlNode::Doctype],
			RsxNode::Comment(comment) => {
				vec![HtmlNode::Comment(comment.value.clone())]
			}
			RsxNode::Text(text) => {
				let str = if self.trim {
					text.value.trim()
				} else {
					&text.value
				};
				vec![HtmlNode::Text(str.into())]
			}
			RsxNode::Element(e) => {
				vec![HtmlNode::Element(self.map_element(idx, e))]
			}
			RsxNode::Fragment(frag) => frag
				.nodes
				.iter()
				.map(|n| self.map_node(n))
				.flatten()
				.collect(),
			RsxNode::Block(rsx_block) => self.map_node(&rsx_block.initial),
			RsxNode::Component(RsxComponent {
				node,
				slot_children,
				..
			}) => {
				if self.no_slot_check == false {
					slot_children.assert_empty();
				}
				// use the location of the root
				let node = self.map_node(&node);
				// even though its empty we must visit to increment
				// the idx incr, in the same order as [`RsxVisitor`] would
				let _ = self.map_node(&slot_children);
				node
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

		let el = HtmlElementNode {
			tag: el.tag.clone(),
			self_closing: el.self_closing,
			attributes: html_attributes,
			children: self.map_node(&el.children),
		};
		el.assert_self_closing_no_children();
		el
	}

	/// Returns a vec to handle the case of [`RsxAttribute::Block`]
	/// which can contain multiple attributes
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
				vec![self.map_maybe_event_attribute(idx, key, Some(initial))]
			}
			RsxAttribute::Block { initial, .. } => initial
				.iter() // test this when implementing
				.map(|(key, value)| {
					self.map_maybe_event_attribute(idx, key, value.as_deref())
				})
				.collect(),
		}
	}

	fn map_maybe_event_attribute(
		&self,
		idx: TreeIdx,
		key: &str,
		value: Option<&str>,
	) -> HtmlAttribute {
		if !self.no_wasm && key.starts_with("on") {
			HtmlAttribute {
				key: key.to_string(),
				value: Some(format!(
					"{}({idx}, event)",
					self.html_constants.event_handler,
				)),
			}
		} else {
			HtmlAttribute {
				key: key.to_string(),
				value: value.map(|v| v.to_string()),
			}
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
				.xpipe(RsxToHtmlString::default())
				.unwrap(),
		)
		.to_be("<!DOCTYPE html>");
	}

	#[test]
	fn comment() {
		expect(
			rsx! { <!-- "hello" --> }
				.xpipe(RsxToHtmlString::default())
				.unwrap(),
		)
		.to_be("<!-- hello -->");
	}

	#[test]
	fn text() {
		expect(rsx! { "hello" }.xpipe(RsxToHtmlString::default()).unwrap())
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
		}.xpipe(RsxToHtmlString::default()).unwrap())
		.to_be("<div name=\"pete\" age=\"9\" favorite_food=\"pizza\" data-beet-rsx-idx=\"1\"></div>");
	}
	#[test]
	fn element_self_closing() {
		expect(rsx! { <br /> }.xpipe(RsxToHtmlString::default()).unwrap())
			.to_be("<br/>");
	}
	#[test]
	fn element_children() {
		expect(
			rsx! { <div>hello</div> }
				.xpipe(RsxToHtmlString::default())
				.unwrap(),
		)
		.to_be("<div>hello</div>");
	}

	#[test]
	fn rsx_text() {
		let value = "hello";
		expect(rsx! { {value} }.xpipe(RsxToHtmlString::default()).unwrap())
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
			.xpipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be("<div><p data-beet-rsx-idx=\"2\">hello mars</p></div>");
	}
	#[test]
	fn block_value_events() {
		let onclick = move |_| {};
		let world = "mars";
		expect(rsx! {
			<div onclick=onclick>
				<p>hello {world}</p>
			</div>
		}.xpipe(RsxToHtmlString::default()).unwrap())
		.to_be("<div onclick=\"_beet_event_handler(1, event)\" data-beet-rsx-idx=\"1\"><p data-beet-rsx-idx=\"2\">hello mars</p></div>");
	}
	#[test]
	fn block_events() {
		#[derive(IntoBlockAttribute)]
		struct Foo {
			onclick: Box<dyn EventHandler<event::MouseEvent>>,
		}

		let world = "mars";
		expect(rsx! {
			<div {Foo{onclick:Box::new(move|_|{})}}>
				<p>hello {world}</p>
			</div>
		}.xpipe(RsxToHtmlString::default()).unwrap())
		.to_be("<div onclick=\"_beet_event_handler(1, event)\" data-beet-rsx-idx=\"1\"><p data-beet-rsx-idx=\"2\">hello mars</p></div>");
	}

	#[test]
	fn component() {
		#[derive(Node)]
		struct Child;
		fn child(_: Child) -> RsxNode {
			rsx! { <p>hello {1}</p> }
		}
		expect(
			rsx! { <Child /> }
				.xpipe(RsxToHtmlString::default())
				.unwrap(),
		)
		.to_be(
			// the component itsself is rsx-idx-0
			"<p data-beet-rsx-idx=\"2\">hello 1</p>",
		);
	}
	#[test]
	fn component_props() {
		#[derive(Node)]
		struct Child {
			value: usize,
		}
		fn child(props: Child) -> RsxNode {
			rsx! { <p>hello {props.value}</p> }
		}

		expect(
			rsx! { <div>the child is <Child value=38 />!</div> }
				.xpipe(RsxToHtmlString::default())
				.unwrap(),
		)
		.to_be(
			"<div>the child is <p data-beet-rsx-idx=\"5\">hello 38</p>!</div>",
		);
	}
	#[test]
	fn component_children() {
		#[derive(Node)]
		struct Layout;
		fn layout(_: Layout) -> RsxNode {
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
			.xpipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be("<div><h1>welcome</h1><p><b>foo</b></p></div>");
	}
	#[test]
	fn component_slots() {
		#[derive(Node)]
		struct Layout;
		fn layout(_: Layout) -> RsxNode {
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
		}.xpipe(RsxToHtmlString::default()).unwrap())
			.to_be("<article><h1>welcome</h1><p><b>what a cool article</b></p><main><div>direct child</div></main></article>");
	}


	#[test]
	fn trims() {
		expect(
			rsx! { "  hello  " }
				.xpipe(RsxToHtmlString {
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

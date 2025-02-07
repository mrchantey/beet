use crate::prelude::*;
use anyhow::Result;
use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::collections::HashMap;


/// This is an RsxNode and a location, which is required for hydration.
///
///
/// The struct returned from an rsx! macro.
#[derive(Debug)]
pub struct RsxRoot {
	/// the root node
	pub node: RsxNode,
	/// unique location with file, line, col
	pub location: RsxLocation,
}

impl RsxRoot {
	/// convenience method usually for testing:
	/// - [ScopedStyle::apply]
	/// - [SlotsVisitor::apply]
	/// - [RsxToHtml::map_node]
	/// - [HtmlNode::render]
	///
	/// # Panics
	/// If the slots cannot be applied.
	pub fn render_body(mut self) -> String {
		ScopedStyle::default().apply(&mut self).unwrap();
		SlotsVisitor::apply(&mut self).unwrap();
		let html = RsxToHtml::default().map_node(&self);
		html.render()
	}


	/// Split the RsxRoot into a template and hydrated nodes.
	pub fn split_hydration(self) -> Result<SplitRsx> {
		let template = RsxTemplateNode::from_rsx_node(&self.node)?;
		let hydrated = RsxHydratedNode::collect(self.node)?;
		let location = self.location;
		Ok(SplitRsx {
			location,
			template,
			hydrated,
		})
	}

	/// Create an RsxRoot from a template and hydrated nodes.
	pub fn join_hydration(
		SplitRsx {
			mut hydrated,
			location,
			template,
		}: SplitRsx,
	) -> Result<Self> {
		let node = RsxTemplateNode::into_rsx_node(template, &mut hydrated)?;

		Ok(Self { node, location })
	}
}

#[derive(Debug)]
pub struct SplitRsx {
	pub location: RsxLocation,
	pub template: RsxTemplateNode,
	pub hydrated: HashMap<RustyTracker, RsxHydratedNode>,
}


impl std::ops::Deref for RsxRoot {
	type Target = RsxNode;
	fn deref(&self) -> &Self::Target { &self.node }
}
impl std::ops::DerefMut for RsxRoot {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.node }
}

impl AsRef<RsxNode> for RsxRoot {
	fn as_ref(&self) -> &RsxNode { &self.node }
}
impl AsMut<RsxNode> for RsxRoot {
	fn as_mut(&mut self) -> &mut RsxNode { &mut self.node }
}

impl Borrow<RsxNode> for RsxRoot {
	fn borrow(&self) -> &RsxNode { &self.node }
}
impl BorrowMut<RsxNode> for RsxRoot {
	fn borrow_mut(&mut self) -> &mut RsxNode { &mut self.node }
}

impl RsxRoot {}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	struct MyComponent {
		value: usize,
	}
	impl Component for MyComponent {
		fn render(self) -> RsxRoot {
			rsx! { <div>the value is {self.value}<slot /></div> }
		}
	}

	// test a roundtrip split/join,
	#[test]
	fn split_join() {
		let some_val = 3;

		let node = || {
			rsx! {
				<div key str="value" num=32 ident=some_val>
					<p>
						hello <MyComponent value=3>
							<div>some child</div>
						</MyComponent>
					</p>
				</div>
			}
		};

		let html1 = node().render_body();
		let split = node().split_hydration().unwrap();
		let node2 = RsxRoot::join_hydration(split).unwrap();
		let html2 = node2.render_body();
		expect(html1).to_be(html2);
	}
	#[test]
	fn split_join_seperate_sources() {
		let some_val = 3;

		let node = rsx! {
			<div key str="value" num=32 ident=some_val>
				<p>
					hello
					<MyComponent value=3>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};
		let node2_template = rsx_template! {
			<div
				key
				str="value"
				num=32
				ident=some_val
				><p>hello
					<MyComponent value=3>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};

		let SplitRsx {
			template: node1_template,
			..
		} = node.split_hydration().unwrap();
		expect(&node1_template).not().to_be(&node2_template);
	}
}

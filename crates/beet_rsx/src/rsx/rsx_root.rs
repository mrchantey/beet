use crate::prelude::*;
use anyhow::Result;
use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::collections::HashMap;

#[derive(Debug)]
pub struct RsxRoot {
	/// in the rsx! macro this will always be a fragment
	pub node: RsxNode,
	pub location: RsxLocation,
}

// type TemplateMap = HashMap<RsxLocation, RsxTemplateNode>;
// type HydratedMap = HashMap<RsxLocation, RsxHydratedNode>;

impl RsxRoot {
	pub fn split(self) -> Result<SplitRsx> {
		let template = RsxTemplateNode::from_rsx_node(&self.node)?;
		let effects = RsxHydratedNode::collect(self.node);
		let location = self.location;
		Ok(SplitRsx {
			location,
			template,
			hydrated: effects,
		})
	}

	pub fn join(mut dehydrated: SplitRsx) -> Result<Self> {
		let node = RsxTemplateNode::into_rsx_node(
			dehydrated.template,
			&mut dehydrated.hydrated,
		)?;

		Ok(Self {
			node,
			location: dehydrated.location,
		})
	}
}

pub struct SplitRsx {
	location: RsxLocation,
	template: RsxTemplateNode,
	hydrated: HashMap<LineColumn, RsxHydratedNode>,
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

	#[test]
	fn from_rsx() {
		let some_val = 3;
		struct MyComponent;
		impl Component for MyComponent {
			fn render(self) -> impl Rsx {
				rsx! {
					<div><slot/></div>
				}
			}
		}

		let node = rsx! {
			<div
				key
				str="value"
				num=32
				ident=some_val
				>
				<p>hello
					<MyComponent>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};

		let html1 = RsxToHtml::render_body(&node);
		let split = node.split().unwrap();

		let node2 = RsxRoot::join(split).unwrap();
		let html2 = RsxToHtml::render_body(&node2);
		println!("html1: {}", html1);
		println!("html2: {}", html2);

		expect(html1).to_be(html2);
	}
}

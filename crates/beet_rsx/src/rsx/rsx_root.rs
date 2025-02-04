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

#[derive(Debug)]
pub struct SplitRsx {
	pub location: RsxLocation,
	pub template: RsxTemplateNode,
	pub hydrated: HashMap<LineColumn, RsxHydratedNode>,
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

		let node = rsx! {
			<div key str="value" num=32 ident=some_val>
				<p>
					hello <MyComponent value=3>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};

		let html1 = RsxToHtml::render_body(&node);
		let split = node.split().unwrap();
		// println!("{:#?}", split);

		let node2 = RsxRoot::join(split).unwrap();
		let html2 = RsxToHtml::render_body(&node2);
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
		let mut node2_template = rsx_template! {
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
			template: mut node1_template,
			..
		} = node.split().unwrap();
		// println!("{:#?}", split);
		node1_template.zero_out_linecol();
		node2_template.zero_out_linecol();
		expect(&node1_template).not().to_be(&node2_template);
	}
}

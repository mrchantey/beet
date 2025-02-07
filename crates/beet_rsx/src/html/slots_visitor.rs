use crate::prelude::*;

/// Slotting is the process of traversing the [RsxComponent::slot_children]
/// and applying them to the [RsxComponent::node] in the corresponding slots.
///
///
/// ```
/// # use beet_rsx::as_beet::*;
///
/// struct MyComponent;
///
/// impl Component for MyComponent {
/// 	fn render(self)->RsxRoot{
/// 		rsx!{
/// 			<html>
/// 				<slot name="header"/>
/// 				<slot/> //default
/// 			</html>
/// 		}
/// 	}
/// }
///
/// let slot_example = rsx!{
/// 	<MyComponent>
///  		<div slot="header">Header</div>
/// 		<div>Default</div>
///  	</MyComponent>
/// };
///
/// let RsxNode::Component(mut component) = slot_example.node else{
/// 	panic!("not a component");
/// };
/// SlotsVisitor::apply_slots(&mut component).unwrap();
/// ```
///
/// # Slot Rules
///
/// - Slot children will be inserted into the first slot with a matching name,
/// 	found via [RsxVisitor] dfs traversal.
/// - Only top level slots, fragments aside, are supported
/// - Any unconsumed slot children will be returned in an error
/// - For unnamed slots `<div/>`, they will be inserted in the components unnamed
/// 	<slot> tag.
/// - Components are not recursively searched because they would steal the slot
/// 	from following internal siblings.
#[derive(Debug)]
pub struct SlotsVisitor {
	default_slots: Vec<RsxNode>,
	named_slots: HashMap<String, Vec<RsxNode>>,
}

#[derive(Debug)]
pub struct SlotsError(pub Vec<(String, Vec<RsxNode>)>);

impl SlotsVisitor {
	/// apply slots to all top level components,
	/// TODO apply recursively to child components?
	pub fn apply(node: &mut RsxNode) -> Result<(), SlotsError> {
		let mut err = None;

		// visit all children
		VisitRsxComponentMut::walk(node, |component| {
			if let Err(e) = Self::apply_to_component(component) {
				err = Some(e);
			}
		});
		if let Some(err) = err {
			Err(err)
		} else {
			Ok(())
		}
	}


	/// apply all slots, returning any named slots that were not consumed
	fn apply_to_component(
		component: &mut RsxComponent,
	) -> Result<(), SlotsError> {
		let mut default_slots = vec![];
		let mut named_slots = HashMap::default();
		// firstly sort slot children
		VisitRsxNodeMut::walk(
			&mut component.slot_children,
			|node| match node {
				RsxNode::Doctype
				| RsxNode::Comment(_)
				| RsxNode::Text(_)
				| RsxNode::Block(_) => {
					// taking a mutable node results in its children not being visited
					default_slots.push(std::mem::take(node));
				}
				RsxNode::Fragment(_) => {
					// println!("fragment");
					// allow traversal
				}
				RsxNode::Element(el) => {
					if let Some(slot_name) = el.get_key_value_attr("slot") {
						let slot_name = slot_name.to_string();
						named_slots
							.entry(slot_name)
							.or_insert_with(Vec::new)
							.push(std::mem::take(node));
					} else {
						default_slots.push(std::mem::take(node));
					}
				}
				RsxNode::Component(_) => {
					// TODO component slot, ie RsxComponent {slot:Option<String>,...}
					default_slots.push(std::mem::take(node));
				}
			},
		);
		// secondly apply the slots
		let mut this = Self {
			default_slots,
			named_slots,
		};
		this.walk_node(&mut component.node);
		let mut unconsumed = this.named_slots.into_iter().collect::<Vec<_>>();

		if !this.default_slots.is_empty() {
			unconsumed.push(("default".to_string(), this.default_slots));
		}
		if unconsumed.is_empty() {
			Ok(())
		} else {
			Err(SlotsError(unconsumed))
		}
	}
}







impl RsxVisitorMut for SlotsVisitor {
	fn ignore_component_node(&self) -> bool {
		// avoid 'slot stealing'
		true
	}
	fn ignore_element_children(&self) -> bool {
		// end if we're done
		self.default_slots.is_empty() && self.named_slots.is_empty()
	}

	/// visit node so we can replace slot with fragment
	fn visit_node(&mut self, node: &mut RsxNode) {
		match node {
			RsxNode::Element(element) => {
				if element.tag == "slot" {
					// println!(
					// 	"visiting slot: \n{:?}\nvisitor:{:?}",
					// 	element, self
					// );
					let nodes = if let Some(name) =
						element.get_key_value_attr("name")
					{
						if let Some(nodes) = self.named_slots.remove(name) {
							nodes
						} else {
							Vec::new()
						}
						// no matching slot children, thats fine
					} else {
						// drains the default slots
						std::mem::take(&mut self.default_slots)
					};
					*node = RsxNode::Fragment(nodes);
				}
			}
			_ => {}
		}
	}
}
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		struct MyComponent;

		impl Component for MyComponent {
			fn render(self) -> RsxRoot {
				rsx! {
					<html>
						<slot name="header"/>
						<slot/> //default
					</html>
				}
			}
		}

		let mut slot_example = rsx! {
			<MyComponent>
				 <div slot="header">Header</div>
				<div>Default</div>
			 </MyComponent>
		};
		SlotsVisitor::apply(&mut slot_example.node).unwrap();
		// println!("{:?}", slot_example);
		expect(RsxToHtml::render_body(slot_example))
			.to_be("<html><div>Header</div><div>Default</div></html>");
	}
	#[test]
	fn recursive() {
		struct MyComponent;

		impl Component for MyComponent {
			fn render(self) -> RsxRoot {
				rsx! {
					<html>
						<slot name="header"/>
						<slot/> //default
					</html>
				}
			}
		}

		let mut slot_example = rsx! {
			<MyComponent>
			<MyComponent>
				 <div slot="header">Header</div>
				<div>Default</div>
				</MyComponent>
			 </MyComponent>
		};
		SlotsVisitor::apply(&mut slot_example.node).unwrap();
		// println!("{:?}", slot_example);
		expect(RsxToHtml::render_body(slot_example)).to_be(
			"<html><html><div>Header</div><div>Default</div></html></html>",
		);
	}
}

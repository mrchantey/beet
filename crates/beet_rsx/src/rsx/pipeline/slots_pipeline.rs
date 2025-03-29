use crate::prelude::*;
use anyhow::Result;
use thiserror::Error;

/// Slotting is the process of traversing the [RsxComponent::slot_children]
/// and applying them to the [RsxComponent::node] in the corresponding slots.
///
/// ```
/// # #![feature(more_qualified_paths)]
/// # use beet_rsx::as_beet::*;
///
/// #[derive(Node)]
/// struct MyComponent;
///
///
/// fn my_component(_: MyComponent)->RsxRoot{
/// 	rsx!{
/// 		<html>
/// 			<slot name="header"/>
/// 			<slot/> //default
/// 		</html>
/// 	}
/// }
///
/// assert_eq!(rsx!{
/// 	<MyComponent>
///  		<div slot="header">Header</div>
/// 		<div>Default</div>
///  	</MyComponent>
/// }.pipe(RsxToHtmlString::default()).unwrap(),
/// "<html><div>Header</div><div>Default</div></html>");
///
/// ```
///
/// ## Slot Rules
///
/// - Slot children will be inserted into the first slot with a matching name,
/// 	found via [RsxVisitor] dfs traversal.
/// - Only top level slots, fragments aside, are supported
/// - Any unconsumed slot children will be returned in an error
/// - For unnamed slots `<div/>`, they will be inserted in the components unnamed
/// 	<slot> tag.
/// - Components are not recursively searched because they would steal the slot
/// 	from following internal siblings.
/// - All <slot> elements are replaced with a <fragment> element containing the
/// 	slot children.
/// - All slot="foo" attributes are removed.
#[derive(Debug, Default, Clone)]
pub struct SlotsPipeline;

impl RsxPipeline<RsxRoot, Result<RsxRoot>> for SlotsPipeline {
	fn apply(self, mut root: RsxRoot) -> Result<RsxRoot> {
		SlotsVisitor::apply(&mut root.node)
			.map(|_| root)
			.map_err(|e| anyhow::anyhow!(e))
	}
}
#[derive(Debug)]
struct SlotsVisitor {
	default_slots: Vec<RsxNode>,
	named_slots: HashMap<String, Vec<RsxNode>>,
}

#[derive(Debug, Error)]
#[error("some slots were not consumed: {unconsumed:?}")]
struct SlotsError {
	unconsumed: Vec<(String, Vec<RsxNodeDiscriminants>)>,
}
impl SlotsError {
	pub fn new(unconsumed: Vec<(String, Vec<RsxNode>)>) -> Self {
		Self {
			unconsumed: unconsumed
				.into_iter()
				.map(|(name, nodes)| {
					(
						name,
						nodes.into_iter().map(|n| n.discriminant()).collect(),
					)
				})
				.collect(),
		}
	}
}

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


	/// apply all slots
	/// ## Errors
	/// If any named slots were not consumed
	fn apply_to_component(
		component: &mut RsxComponent,
	) -> Result<(), SlotsError> {
		let mut default_slots = vec![];
		let mut named_slots = HashMap::default();
		// firstly sort slot children
		VisitRsxNodeMut::walk(
			&mut component.slot_children,
			|node| match node {
				RsxNode::Doctype { .. }
				| RsxNode::Comment { .. }
				| RsxNode::Text { .. }
				| RsxNode::Block(_) => {
					// taking a mutable node results in its children not being visited
					default_slots.push(std::mem::take(node));
				}
				RsxNode::Fragment { .. } => {
					// println!("fragment");
					// allow traversal
				}
				RsxNode::Element(el) => {
					if let Some(slot_name) = el.get_key_value_attr("slot") {
						let slot_name = slot_name.to_string();
						// remove the slot attribute
						el.remove_matching_key("slot");
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
		this.walk_node(&mut component.root);
		let mut unconsumed = this.named_slots.into_iter().collect::<Vec<_>>();

		if !this.default_slots.is_empty() {
			unconsumed.push(("default".to_string(), this.default_slots));
		}
		if unconsumed.is_empty() {
			Ok(())
		} else {
			Err(SlotsError::new(unconsumed))
		}
	}
}







impl RsxVisitorMut for SlotsVisitor {
	fn ignore_component_node(&self) -> bool {
		// avoid 'slot stealing'
		true
	}
	// we cant exit early because we still need to find and remove default <slot/>
	fn ignore_element_children(&self) -> bool { false }

	/// visit node so we can replace slot with fragment of same idx
	fn visit_node(&mut self, node: &mut RsxNode) {
		// println!("visiting node: {:?}", node);

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
						// no matching slot children, thats allowed
					} else {
						// drains the default slots
						std::mem::take(&mut self.default_slots)
					};
					*node = RsxNode::Fragment { nodes };
				}
			}
			_ => {}
		}
	}
}
#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		#[derive(Node)]
		struct MyComponent;

		fn my_component(_: MyComponent) -> RsxRoot {
			rsx! {
				<html>
					<slot name="header" />
					// default
					<slot />
				</html>
			}
		}

		// println!("{:?}", slot_example);
		expect(
			rsx! {
				<MyComponent>
					<div slot="header">Header</div>
					<div>Default</div>
				</MyComponent>
			}
			.pipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be("<html><div>Header</div><div>Default</div></html>");
	}
	#[test]
	fn recursive() {
		#[derive(Node)]
		struct MyComponent;

		fn my_component(_: MyComponent) -> RsxRoot {
			rsx! {
				<html>
					<slot name="header" />
					// default
					<slot />
				</html>
			}
		}

		expect(
			rsx! {
				<MyComponent>
					<MyComponent>
						<div slot="header">Header</div>
						<div>Default</div>
					</MyComponent>
				</MyComponent>
			}
			.pipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be("<html><html><div>Header</div><div>Default</div></html></html>");
	}
}

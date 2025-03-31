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
/// fn my_component(_: MyComponent)->RsxNode {
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
/// }.bpipe(RsxToHtmlString::default()).unwrap(),
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
pub struct ApplySlots;

impl RsxPipeline<RsxNode, Result<RsxNode, SlotsError>> for ApplySlots {
	fn apply(self, mut node: RsxNode) -> Result<RsxNode, SlotsError> {
		let mut root_slot_map = HashMap::default();
		Self::map_node(&mut node, &mut root_slot_map)?;
		// the root slot map should never be filled
		slot_map_to_result(root_slot_map).map(|_| node)
	}
}

impl ApplySlots {
	/// visit a node tree from the bottom up, collecting all slots and applying them to the node.
	/// any slots for the parent are returned.
	/// # Errors
	/// - If [`ApplySlots::apply_to_node`] returns an error, in which case
	/// a non-bubbling slot was not consumed.
	/// - If [`ApplySlots::collect_slot_map`]
	fn map_node(
		node: &mut RsxNode,
		// only for the apply step, passing up any bubble slots
		parent_slot_map: &mut HashMap<String, Vec<RsxNode>>,
	) -> Result<(), SlotsError> {
		// this is the 'parent_slot_map' for this nodes descendents
		// it is used to collect any bubbling slots
		let mut slot_map = HashMap::default();
		// println!(
		// 	"visiting node start: {:?} - {}",
		// 	node.discriminant(),
		// 	node.location_str()
		// );

		match node {
			RsxNode::Doctype(_) | RsxNode::Comment(_) | RsxNode::Text(_) => {}
			RsxNode::Block(rsx_block) => {
				Self::map_node(&mut rsx_block.initial, &mut slot_map)?;
			}
			RsxNode::Fragment(rsx_fragment) => {
				for child in &mut rsx_fragment.nodes {
					Self::map_node(child, &mut slot_map)?;
				}
			}
			RsxNode::Element(el) => {
				Self::map_node(&mut el.children, &mut slot_map)?;
			}
			RsxNode::Component(component) => {
				println!("processing component: {}", component.tag);
				Self::map_node(&mut component.node, &mut slot_map)?;
				// this feels weird, slot children are a special case in ApplySlot
				// but we're treating them like normal, but its all working atm i guess
				Self::map_node(&mut component.slot_children, &mut slot_map)?;

				// First collect all available slots from this component's slot_children
				let component_slots =
					Self::collect_slot_children(&mut component.slot_children);
				println!(
					"component slots: {:?}",
					slot_map_debug(&component_slots)
				);
				slot_map.extend(component_slots);

				
				println!("full slot_map: {:?}", slot_map_debug(&slot_map));
				
				

				Self::apply_to_node(
					&mut component.node,
					slot_map,
					parent_slot_map,
				)?;
			}
		}
		println!(
			"visiting node end:   {:?} - {}",
			node.discriminant(),
			node.location_str()
		);


		Ok(())
	}


	/// collect any child elements with a slot attribute into a hashmap
	/// of that name, ie <div slot="foo">, and remove the slot attribute
	fn collect_slot_children(
		children: &mut RsxNode,
	) -> HashMap<String, Vec<RsxNode>> {
		let mut slot_map = HashMap::default();
		let mut insert = |name: &str, node: &mut RsxNode| {
			slot_map
				.entry(name.to_string())
				.or_insert_with(Vec::new)
				// taking a mutable node results in its children not being visited
				.push(std::mem::take(node));
		};

		// apply all slot children either to the default slot or its named slot
		// using a visitor handles the case of nested fragments
		VisitRsxNodeMut::walk_with_opts(
			children,
			// top level only
			VisitRsxOptions::ignore_all(),
			|node| match node {
				RsxNode::Doctype { .. }
				| RsxNode::Comment { .. }
				| RsxNode::Text { .. }
				| RsxNode::Block(_) => {
					insert("default", node);
				}
				RsxNode::Fragment { .. } => {
					// allow traversal
				}
				RsxNode::Element(el) => {
					let slot_name = el
						.get_key_value_attr("slot")
						.unwrap_or("default")
						.to_string();
					// remove the slot attribute if it exists
					el.remove_matching_key("slot");
					insert(&slot_name, node);
				}
				RsxNode::Component(_) => {
					// TODO component slot field, ie RsxComponent {slot:Option<String>,...}
					insert("default", node);
				}
			},
		);
		slot_map
	}

	/// Apply the slots to the node.
	/// ## Bubbling up
	/// The returned hashmap contains the children of
	/// any <slot slot="foo"> where the key is "foo" and the value is
	/// the *resolved* children of the slot.
	/// ## Errors
	///
	/// If there are any unconsumed slots, an error is returned
	fn apply_to_node(
		node: &mut RsxNode,
		mut slot_map: HashMap<String, Vec<RsxNode>>,
		// the slot map for the parent component, to be filled up with
		// any bubbling slots
		parent_slot_map: &mut HashMap<String, Vec<RsxNode>>,
	) -> Result<(), SlotsError> {
		VisitRsxNodeMut::walk_with_opts(
			node,
			// avoid 'slot stealing' by not visiting any child component nodes
			// using a visitor handles element children (just to remove the <slot>)
			// and fragments
			VisitRsxOptions::ignore_component_node(),
			|node| {
				match node {
					RsxNode::Element(element) => {
						if element.tag == "slot" {
							let name = element
								.get_key_value_attr("name")
								.unwrap_or("default");
							// no matching slot children is allowed, so use default
							// TODO fallback to using the slots children https://docs.astro.build/en/basics/astro-components/#fallback-content-for-slots
							let nodes =
								slot_map.remove(name).unwrap_or_default();
							if let Some(bubbling_name) =
								element.get_key_value_attr("slot")
							{
								// its a <slot slot="foo"> ie a bubbling slot,
								// so add the children to the parent slot map and replace
								// with default
								parent_slot_map
									.entry(bubbling_name.to_string())
									.or_default()
									.extend(nodes);
								*node = RsxNode::default();
							} else {
								*node = nodes.into_node();
							}
						}
					}
					_ => {}
				}
			},
		);
		slot_map_to_result(slot_map)
	}
}

fn slot_map_debug(map: &HashMap<String, Vec<RsxNode>>) -> String {
	let mut s = String::new();
	for (name, nodes) in map {
		s.push_str(&format!(
			"{}: {:?}\n",
			name,
			nodes.iter().map(|n| n.discriminant()).collect::<Vec<_>>()
		));
	}
	s
}

/// if the hashmap is empty, return Ok(()), otherwise return an error
fn slot_map_to_result(
	map: HashMap<String, Vec<RsxNode>>,
) -> Result<(), SlotsError> {
	let unconsumed = map.into_iter().collect::<Vec<_>>();
	if unconsumed.is_empty() {
		Ok(())
	} else {
		Err(SlotsError::new(unconsumed))
	}
}

#[derive(Debug, Error)]
#[error("some slots were not consumed: {unconsumed:?}")]
pub struct SlotsError {
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
#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		#[derive(Node)]
		struct MyComponent;

		fn my_component(_: MyComponent) -> RsxNode {
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
			.bpipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be("<html><div>Header</div><div>Default</div></html>");
	}
	#[test]
	fn recursive() {
		#[derive(Node)]
		struct MyComponent;

		fn my_component(_: MyComponent) -> RsxNode {
			rsx! {
				<html>
					<slot name="header" />
					<br />
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
			.bpipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be(
			"<html><br/><html><div>Header</div><br/><div>Default</div></html></html>",
		);
	}



	#[test]
	fn bubbles() {
		#[derive(Node)]
		struct Parent;

		fn parent(_: Parent) -> RsxNode {
			rsx! {
				<slot name="header" />
				<slot/>
			}
		}
		#[derive(Node)]
		struct Child;

		fn child(_: Child) -> RsxNode {
			rsx! {
					<slot name="header" slot="header" />
			}
		}
		expect(
			rsx! {
				<Parent>
					<Child>
						<div slot="header">My App</div>
					</Child>
				</Parent>
			}
			.bpipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be("<div>My App</div>");
	}

	#[test]
	fn complex_bubbles() {
		#[derive(Node)]
		struct Layout;

		fn layout(_: Layout) -> RsxNode {
			rsx! {
				<p>
					<Header>
						<slot name="header" />
					</Header>
					<slot/>
				</p>
			}
		}

		#[derive(Node)]
		struct Header;

		fn header(_: Header) -> RsxNode {
			rsx! {
				<header>
					<slot/>
				</header>
			}
		}


		expect(
			rsx! {
				<Layout>
					<div slot="header">My App</div>
					<div>Content</div>
				</Layout>
			}
			.bpipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be("<header><div>My App</div><p><div>Content</div></p></header>");
	}
}

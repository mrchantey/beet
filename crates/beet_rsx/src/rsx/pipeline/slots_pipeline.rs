use crate::prelude::*;
use anyhow::Result;
use rapidhash::RapidHashMap;
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
pub struct SlotsPipeline;

impl RsxPipeline<RsxNode, Result<RsxNode, SlotsError>> for SlotsPipeline {
	/// apply slots to all top level components,
	fn apply(self, mut node: RsxNode) -> Result<RsxNode, SlotsError> {
		let mut err = Ok(());

		// visit all children
		VisitRsxComponentMut::walk(&mut node, |component| {
			let slot_map = Self::collect_slot_map(component);
			if let Err(e) = Self::apply_to_node(&mut component.node, slot_map) {
				err = Err(e);
			}
		});
		err.map(|_| node)
	}
}

impl SlotsPipeline {
	/// collect any child elements with a slot attribute,
	/// ie <div slot="foo">
	fn collect_slot_map(
		component: &mut RsxComponent,
	) -> HashMap<String, Vec<RsxNode>> {
		let mut slot_map = HashMap::default();
		let mut insert = |name: &str, node: &mut RsxNode| {
			slot_map
				.entry(name.to_string())
				.or_insert_with(Vec::new)
				// taking a mutable node results in its children not being visited
				.push(std::mem::take(node));
		};

		// firstly sort slot children
		VisitRsxNodeMut::walk(
			&mut component.slot_children,
			// top level only
			// VisitRsxOptions::default(),
			// VisitRsxOptions::ignore_all(),
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
					// TODO component slot, ie RsxComponent {slot:Option<String>,...}
					insert("default", node);
				}
			},
		);
		slot_map
	}

	/// secondly apply the slots
	/// visit node so we can replace slot with fragment
	fn apply_to_node(
		node: &mut RsxNode,
		mut slot_map: RapidHashMap<String, Vec<RsxNode>>,
	) -> Result<(), SlotsError> {
		VisitRsxNodeMut::walk_with_opts(
			node,
			// avoid 'slot stealing' by not visiting any child component nodes
			// we still need to visit element children because we still need to find and remove default <slot/>
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
							// handle bubbling
							if let Some(_slot_name) =
								element.get_key_value_attr("slot")
							{
								// unimplemented!("bubbling");
								*node = nodes.into_node();
							} else {
								*node = nodes.into_node();
							}
						}
					}
					_ => {}
				}
			},
		);
		let unconsumed = slot_map.into_iter().collect::<Vec<_>>();
		if unconsumed.is_empty() {
			Ok(())
		} else {
			Err(SlotsError::new(unconsumed))
		}
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
					<br/>
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
	#[ignore = "we need reverse visitors"]
	fn bubbles() {
		#[derive(Node)]
		struct Comp1;

		fn comp1(_: Comp1) -> RsxNode {
			rsx! {
				<header>
					<slot name="header" />
					<slot/>
				</header>
			}
		}
		#[derive(Node)]
		struct Comp2;

		fn comp2(_: Comp2) -> RsxNode {
			rsx! {
				<header>
					<slot name="header" slot="header" />
				</header>
			}
		}

		expect(
			rsx! {
				<Comp1>
					<Comp2>
						<div slot="header">My App</div>
					</Comp2>
				</Comp1>
			}
			.bpipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be("<html><div>Header</div><html><div>Default</div></html></html>");
	}
}

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
/// 	<sag.
/// - Comnot recursively searched because they would steal the slot
/// 	from following internal siblings.
/// - All <slot> elements are replaced with a <fragment> element containing the
/// 	slot children.
/// - All slot="foo" attributes are removed.
#[derive(Debug, Default, Clone)]
pub struct ApplySlots;

impl RsxPipeline<RsxNode, Result<RsxNode, SlotsError>> for ApplySlots {
	fn apply(self, mut node: RsxNode) -> Result<RsxNode, SlotsError> {
		Self::map_node(&mut node).map(|_| node)
	}
}

impl ApplySlots {
	/// Apply slots for a given node, if it isnt an [`RsxComponent`] this is a noop.
	/// 1. Collect all [`RsxComponent::slot_children`] into a hashmap, any direct
	/// 	 children without a slot directive are added to the "default" slot.
	fn map_node(node: &mut RsxNode) -> Result<(), SlotsError> {
		let mut res = Ok(());
		VisitRsxNodeMut::walk_with_opts(
			node,
			VisitRsxOptions::default(),
			|node| {
				let RsxNode::Component(component) = node else {
					return;
				};
				let slot_map =
					Self::collect_slot_children(&mut component.slot_children);
				if let Err(err) =
					Self::insert_slot_children(&mut *component.node, slot_map)
				{
					res = Err(err);
				}
			},
		);
		Ok(())
	}

	/// collect any *direct descendent* child elements with a slot directive into a hashmap
	/// of that name, ie <div slot="foo">, and remove the slot directive
	///
	/// # Panics
	///
	/// If the slot children are still not empty after the visit. this is
	/// an internal error, file a bug report
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
			|node| {
				match node {
					RsxNode::Doctype(_)
					| RsxNode::Comment(_)
					| RsxNode::Text(_)
					| RsxNode::Block(_)
					| RsxNode::Component(_)
					| RsxNode::Element(_) => {
						let slot_name = node
							.slot_directive()
							.map(|d| d.to_string())
							.unwrap_or_else(|| "default".to_string());
						insert(&slot_name, node);
					}
					RsxNode::Fragment(fragment) => {
						// only apply to fragment if it has a slot directive
						// otherwise allow traversal
						if let Some(slot_name) = fragment.slot_directive() {
							insert(&slot_name.to_string(), node);
						}
					}
				}
			},
		);
		children.assert_empty();
		slot_map
	}
	/// Apply slot map to all <slot> elements in the following places:
	/// - top level children of the component
	/// - element children (recursive)
	/// - fragment children (recursive)
	/// - child component slot children (recursive)
	fn insert_slot_children(
		node: &mut RsxNode,
		mut slot_map: HashMap<String, Vec<RsxNode>>,
	) -> Result<(), SlotsError> {
		// visit node so we can set it
		VisitRsxNodeMut::walk_with_opts(
			node,
			// only visit element and fragment children
			VisitRsxOptions {
				ignore_element_children: false,
				ignore_block_node_initial: true,
				ignore_component_node: true,
				ignore_component_slot_children: false,
				bottom_up: false,
			},
			|node| {
				let RsxNode::Element(element) = node else {
					return;
				};
				if element.tag != "slot" {
					return;
				}
				let slot_name =
					element.get_key_value_attr("name").unwrap_or("default");
				// println!("inserting slot children\nslot name: {}", slot_name);
				// println!("slot map: {:#?}", slot_map);
				let slot_children = slot_map
					.remove(slot_name)
					.map(|vec| vec.into_node())
					// <slot>foo</slot> fallback pattern https://docs.astro.build/en/basics/astro-components/#fallback-content-for-slots
					.unwrap_or(std::mem::take(&mut *element.children));
				element.remove_matching_key("name");
				// println!("slot children: {:#?}", slot_children);

				// <slot slot="foo"> transfer pattern https://docs.astro.build/en/basics/astro-components/#transferring-slots
				// in this case we simply convert to a fragment
				// it will be collected in the [`Self::collect_slot_children`] of the child
				if element.slot_directive().is_some() {
					*node = RsxFragment {
						nodes: vec![slot_children],
						meta: element.meta().clone(),
					}
					.into_node();
				// println!("node: {:?}", node);
				// println!(
				// 	"after: {:}",
				// 	node.bpipe(RsxToHtml::default())
				// 		.bpipe(RenderHtml::default())
				// 		.unwrap()
				// );
				} else {
					*node = slot_children;
				}
			},
		);
		slot_map_to_result(slot_map)
	}
}

#[allow(unused)]
fn slot_map_debug(map: &HashMap<String, Vec<RsxNode>>) -> String {
	let mut s = String::new();
	for (name, nodes) in map {
		s.push_str(&format!(
			"{}: {:?}, ",
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

	#[derive(Node)]
	struct Span;

	fn span(_: Span) -> RsxNode {
		rsx! {
			<span>
				<slot />
			</span>
		}
	}

	#[derive(Node)]
	struct MyComponent;

	fn my_component(_: MyComponent) -> RsxNode {
		rsx! {
			<html>
				<slot name="header">Fallback Title</slot>
				<br />
				// default
				<slot />
			</html>
		}
	}

	#[test]
	fn works() {
		expect(
			rsx! {
				<MyComponent>
					<div>Default</div>
					<div slot="header">Title</div>
				</MyComponent>
			}
			.bpipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be("<html><div>Title</div><br/><div>Default</div></html>");
	}

	#[test]
	fn component_slots() {
		expect(
			rsx! {
				<MyComponent>
					<div>Default</div>
					<Span slot="header">Title</Span>
				</MyComponent>
			}
			.bpipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be("<html><span>Title</span><br/><div>Default</div></html>");
	}



	#[test]
	fn fallback() {
		expect(
			rsx! { <MyComponent /> }
				.bpipe(RsxToHtmlString::default())
				.unwrap(),
		)
		.to_be("<html>Fallback Title<br/></html>");
	}

	#[test]
	fn recursive() {
		expect(
			rsx! {
				<Span>
					<MyComponent>
						<div>Default</div>
						<div slot="header">Title</div>
					</MyComponent>
				</Span>
			}
			.bpipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be(
			"<span><html><div>Title</div><br/><div>Default</div></html></span>",
		);
	}

	#[test]
	fn transfer_simple() {
		#[derive(Node)]
		struct Layout;

		fn layout(_: Layout) -> RsxNode {
			rsx! {
				<Header>
					<slot name="header" slot="default" />
				</Header>
			}
		}
		#[derive(Node)]
		struct Header;

		fn header(_: Header) -> RsxNode {
			rsx! {
				<header>
					<slot />
				</header>
			}
		}
		expect(
			rsx! {
				<Layout>
					<h1 slot="header">"Title"</h1>
				</Layout>
			}
			.bpipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be("<header><h1>Title</h1></header>");
	}

	#[test]
	fn transfer_complex() {
		#[derive(Node)]
		struct Layout;

		fn layout(_: Layout) -> RsxNode {
			rsx! {
				<body>
					<Header>
						<slot name="header" slot="default" />
					</Header>
					<main>
						<slot />
					</main>
				</body>
			}
		}

		#[derive(Node)]
		struct Header;

		fn header(_: Header) -> RsxNode {
			rsx! {
				<header>
					<slot />
				</header>
			}
		}


		expect(
			rsx! {
				<Layout>
					<div>"Content"</div>
					<h1 slot="header">"Title"</h1>
				</Layout>
			}
			.bpipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be("<body><header><h1>Title</h1></header><main><div>Content</div></main></body>");
	}
}

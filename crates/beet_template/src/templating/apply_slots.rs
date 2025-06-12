//! Slotting is the process of traversing the [RsxComponent::slot_children]
//! and applying them to the [RsxComponent::node] in the corresponding slots.
//!
//! ## Example
//! ```
//! # use beet_template::as_beet::*;
//! # use bevy::prelude::*;
//!
//!
//! #[template]
//! fn MyComponent() -> impl Bundle {
//! 	rsx!{
//! 		<html>
//! 			<slot name="header"/>
//! 			<slot/> //default
//! 		</html>
//! 	}
//! }
//! assert_eq!(
//! 	HtmlFragment::parse_bundle(rsx!{
//! 		<MyComponent>
//!  			<div slot="header">Header</div>
//! 			<div>Default</div>
//!  		</MyComponent>
//! 	}),
//! 	"<html><div>Header</div><div>Default</div></html>"
//! );
//!
//! ```
//!
//! ## Slot Rules
//!
//! - Slot children will be inserted into the first slot with a matching name
//! - Only top level slots are supported to avoid 'slot stealing'
//! - Any unconsumed slot children will return in an error
//! - For unnamed slots `<div/>`, they will be inserted in the components default <slot/>
//! - All <slot> elements are replaced with a <fragment> element containing the
//! 	slot children.
//! - 'Slot Transfers' are supported, ie <slot name="header" slot="default"/>
//!   see https://docs.astro.build/en/basics/astro-components/#transferring-slots
use crate::prelude::*;
use beet_common::node::SlotChild;
use beet_common::node::SlotTarget;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use sweet::prelude::HierarchyQueryExtExt;

/// Applying slots has several steps:
/// 1. Collect all slot children.
/// 2. Collect all slot targets
/// 3. For each slot, apply to the slot target.
/// 4. Move the [`TemplateRoot`] relation to a [`Children`].
/// 5. Remove any fallback children from slot targets which are used.
pub(super) fn apply_slots(
	mut commands: Commands,
	children: Query<&Children>,
	slot_targets: Query<(Entity, &SlotTarget)>,
	slot_children: Query<(Entity, &SlotChild)>,
	query: Populated<(Entity, &TemplateRoot)>,
) -> Result {
	for (node_ent, root) in query.iter() {
		let (named_slots, default_slots) =
			collect_slot_children(node_ent, &children, &slot_children);

		// 2.a Collect all named slot targets
		let slot_targets = children
			.iter_descendants(**root)
			.filter_map(|c| slot_targets.get(c).ok())
			.collect::<Vec<_>>();

		// 2.b Find the default slot targets
		let default_slot_target = slot_targets
			.iter()
			.find(|(_, target)| **target == SlotTarget::Default);

		let mut used_targets = HashSet::<Entity>::default();

		// 3.a Apply named slots
		for (named_slot_ent, named_slot) in named_slots.iter() {
			let Some((target, _)) = slot_targets
				.iter()
				.find(|(_, target)| target.name() == Some(named_slot.as_str()))
			else {
				return Err(anyhow::anyhow!(
					"Named slot `{}` found but no matching target found for {node_ent:?}",
					named_slot
				)
				.into());
			};
			used_targets.insert(*target);
			commands.entity(*named_slot_ent).insert(ChildOf(*target));
		}

		// 3.b Apply default slots
		if !default_slots.is_empty() {
			let Some((target, _)) = default_slot_target else {
				return Err(anyhow::anyhow!(
					"Default slot found but no default slot target found for {node_ent:?}"
				)
				.into());
			};
			used_targets.insert(*target);
			for slot in default_slots {
				commands.entity(slot).insert(ChildOf(*target));
			}
		}

		// 4. move the template root to the children
		commands
			.entity(**root)
			.remove::<TemplateOf>()
			.insert(ChildOf(node_ent));
		// 5. remove fallback children from used targets
		used_targets
			.into_iter()
			.filter_map(|e| children.get(e).ok())
			.for_each(|children| {
				for child in children.iter() {
					commands.entity(child).despawn();
				}
			});
	}
	Ok(())
}

fn collect_slot_children(
	node_ent: Entity,
	children: &Query<&Children>,
	slot_children: &Query<(Entity, &SlotChild)>,
) -> (Vec<(Entity, String)>, Vec<Entity>) {
	// 1. Collect all named slots
	// all children with a slot directive, ie <div slot="foo"/>
	// this must be direct descendants only to avoid 'slot stealing'
	// where a templates parent ends up trying to resolve the slot
	// see the recursive test
	let named_slots = children
		.iter_direct_descendants(node_ent)
		.filter_map(|c| slot_children.get(c).ok())
		.collect::<Vec<_>>();

	// 2. Collect all default slots
	// all direct descendants with no slot directive, ie <div/>
	let mut default_slots = children
		.get(node_ent)
		.map(|children| {
			children
				.iter()
				.filter(|e| !named_slots.iter().any(|s| s.0 == *e))
				.collect::<Vec<_>>()
		})
		.unwrap_or_default();

	// 3. move named slots to default
	let named_slots = named_slots
		.into_iter()
		.filter_map(|(entity, slot)| {
			if let SlotChild::Named(name) = slot {
				Some((entity, name.to_string()))
			} else {
				default_slots.push(entity);
				None
			}
		})
		.collect::<Vec<_>>();

	(named_slots, default_slots)
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[template]
	fn Span() -> impl Bundle {
		rsx! {
			<span>
			<slot />
			</span>
		}
	}

	#[template]
	fn MyComponent() -> impl Bundle {
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
		rsx! {
			<MyComponent>
				<div>Default</div>
				<div slot="header">Title</div>
			</MyComponent>
		}
		.xmap(HtmlFragment::parse_bundle)
		.xpect()
		.to_be("<html><div>Title</div><br/><div>Default</div></html>");
	}

	#[test]
	fn component_slots() {
		rsx! {
			<MyComponent>
				<div>Default</div>
				<Span slot="header">Title</Span>
			</MyComponent>
		}
		.xmap(HtmlFragment::parse_bundle)
		.xpect()
		.to_be("<html><span>Title</span><br/><div>Default</div></html>");
	}

	#[test]
	fn fallback() {
		rsx! { <MyComponent /> }
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<html>Fallback Title<br/></html>");
	}

	#[test]
	fn recursive() {
		rsx! {
			<Span>
				<MyComponent>
					<div>Default</div>
					<div slot="header">Title</div>
				</MyComponent>
			</Span>
		}
		.xmap(HtmlFragment::parse_bundle)
		.xpect()
		.to_be(
			"<span><html><div>Title</div><br/><div>Default</div></html></span>",
		);
	}

	#[test]
	fn transfer_simple() {
		#[template]
		fn Layout() -> impl Bundle {
			rsx! {
			<Header>
			<slot name="header" slot="default" />
			</Header>
				}
		}

		#[template]
		fn Header() -> impl Bundle {
			rsx! {
				<header>
					<slot />
				</header>
			}
		}
		rsx! {
			<Layout>
				<h1 slot="header">"Title"</h1>
			</Layout>
		}
		.xmap(HtmlFragment::parse_bundle)
		.xpect()
		.to_be("<header><h1>Title</h1></header>");
	}

	#[test]
	fn transfer_complex() {
		#[template]
		fn Layout() -> impl Bundle {
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

		#[template]
		fn Header() -> impl Bundle {
			rsx! {
				<header>
					<slot />
				</header>
			}
		}

		rsx! {
			<Layout>
				<div>"Content"</div>
				<h1 slot="header">"Title"</h1>
			</Layout>
		}
		.xmap(HtmlFragment::parse_bundle)
		.xpect()
		.to_be("<body><header><h1>Title</h1></header><main><div>Content</div></main></body>");
	}
}

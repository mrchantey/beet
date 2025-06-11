use crate::prelude::*;
use beet_common::as_beet::*;
use bevy::prelude::*;


/// Currently only [`EventObserver`] and [`TextNodeParent`] elements are
/// the only ones that require a [`TreeIdx`] attribute
// see render_html.rs for tests
pub(super) fn apply_tree_idx(
	mut commands: Commands,
	html_constants: Res<HtmlConstants>,
	requires_tree_idx_attr: Populated<
		Entity,
		Or<(Added<EventObserver>, Added<TextNodeParent>)>,
	>,
) {
	let mut id = 0;

	for entity in requires_tree_idx_attr.iter() {
		commands.entity(entity).insert(TreeIdx::new(id));

		commands.spawn((
			AttributeOf::new(entity),
			AttributeKeyStr::new(html_constants.tree_idx_key.clone()),
			AttributeValueStr::new(id.to_string()),
		));
		id += 1;
	}
}

/// Similar to an [`Entity`], contaning a unique identifier for this node in
/// a templating tree. Unlike [`Entity`] this id will always be the same no matter
/// how many other existing entities in the world.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct TreeIdx(
	/// Breadth-first index of this node in the templating tree.
	pub u32,
);


impl TreeIdx {
	pub fn new(idx: u32) -> Self { Self(idx) }
}




#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn applies_ids() {
		let mut world = World::new();
		world.init_resource::<HtmlConstants>();
		let (get,_set) = signal(2);
		let entity = world
			.spawn((
				rsx! {
					<div onclick=||{}>
						"child 1"
						<span>"child with signal"{get}</span>
						"child 2"
					</div>
				},
				ToHtml,
			))
			.id();
		world.run_system_once(super::super::apply_text_node_parents).unwrap();
		world.run_system_once(super::apply_tree_idx).unwrap();

		world
			.get::<TreeIdx>(entity)
			.unwrap()
			.xpect()
			.to_be(&TreeIdx(0));

		let children = world.get::<Children>(entity).unwrap();
		world
			.get::<TreeIdx>(children[1])
			.unwrap()
			.xpect()
			.to_be(&TreeIdx(1));
	}
}

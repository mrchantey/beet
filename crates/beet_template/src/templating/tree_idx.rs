use crate::prelude::*;
use beet_common::as_beet::*;
use bevy::prelude::*;
use sweet::prelude::HierarchyQueryExtExt;


pub(super) fn apply_tree_idx(
	mut commands: Commands,
	query: Populated<Entity, Added<ToHtml>>,
	children: Query<&Children>,
) {
	for root in query.iter() {
		let mut id = 0;
		// bfs traversal of the tree
		for child in children.iter_descendants_inclusive(root) {
			commands.entity(child).insert(TreeIdx::new(id));
			id += 1;
		}
	}
}

/// Currently only [`EventObserver`] and [`TextNodeParent`] elements are
/// the only ones that require a [`TreeIdx`] attribute
// see render_html.rs for tests
pub(super) fn add_tree_idx_attributes(
	mut commands: Commands,
	html_constants: Res<HtmlConstants>,
	requires_tree_idx_attr: Query<
		(Entity, &TreeIdx),
		Or<(With<EventObserver>, With<TextNodeParent>)>,
	>,
) {
	for (entity, tree_idx) in requires_tree_idx_attr.iter() {
		commands.spawn((
			AttributeOf::new(entity),
			AttributeKeyStr::new(html_constants.tree_idx_key.clone()),
			AttributeValueStr::new(tree_idx.0.to_string()),
		));
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
	fn applies_bfs_ids() {
		let mut world = World::new();
		let entity = world
			.spawn((
				rsx! {
					<div>
						"child 1"
						<span>"nested child"</span>
						"child 2"
					</div>
				},
				ToHtml,
			))
			.id();
		world.run_system_once(super::apply_tree_idx).unwrap();

		world
			.get::<TreeIdx>(entity)
			.unwrap()
			.xpect()
			.to_be(&TreeIdx(0));

		let children = world.get::<Children>(entity).unwrap();
		world
			.get::<TreeIdx>(children[0])
			.unwrap()
			.xpect()
			.to_be(&TreeIdx(1));
		world
			.get::<TreeIdx>(children[1])
			.unwrap()
			.xpect()
			.to_be(&TreeIdx(2));
		world
			.get::<TreeIdx>(children[2])
			.unwrap()
			.xpect()
			.to_be(&TreeIdx(3));

		let nested_children = world.get::<Children>(children[1]).unwrap();
		world
			.get::<TreeIdx>(nested_children[0])
			.unwrap()
			.xpect()
			.to_be(&TreeIdx(4));
	}
}

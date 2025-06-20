use crate::prelude::*;
use beet_bevy::bevybail;
use beet_common::node::TextNode;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// The fs loaded and deduplicated [`LangContent`], existing seperately from the
/// originating tree(s).
#[derive(Debug, Clone, PartialEq, Hash, Deref, Component, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component)]
// #[component(immutable)]
pub struct LangPartial(pub String);

impl LangPartial {
	/// Create a new [`LangPartial`] from a `String`.
	pub fn new(content: String) -> Self { Self(content) }
}


/// For trees with [`PortalTo<LangPartial`], insert a single element at the top
/// of the tree, to be hoisted to the head.
pub fn resolve_lang_partials(
	mut commands: Commands,
	partials: Query<(Entity, &LangPartial)>,
	parents: Query<&ChildOf>,
	query: Populated<(Entity, &NodePortal), With<PortalTo<LangPartial>>>,
) -> Result {
	let mut root_content = HashMap::<Entity, HashMap<Entity, String>>::new();

	for (entity, portal) in query.iter() {
		let Ok(partial) = partials.get(**portal) else {
			bevybail!(
				"NodePortal is missing a target LangPartial: {:?}",
				**portal
			);
		};
		root_content
			.entry(parents.root_ancestor(entity))
			.or_default()
			.insert(partial.0, partial.1.0.clone());
	}

	for (root, partials) in root_content.into_iter() {
		for (partial_entity, contents) in partials.into_iter() {
			// insert as direct child of root
			commands
				.entity(partial_entity)
				.clone_and_spawn()
				.remove::<LangPartial>()
				.insert((ChildOf(root), children![TextNode::new(contents)]));
		}
	}

	Ok(())
}

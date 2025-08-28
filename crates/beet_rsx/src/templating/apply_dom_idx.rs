use crate::prelude::*;
use beet_core::as_beet::*;
use bevy::prelude::*;

/// Some cases cant just use a `#[requires(RequiresDomIdx)]`,
/// like the parent element of a dynamic attribute, fragment or text node.
/// This system applies the [RequiresDomIdx] attribute to those entities.
pub fn apply_requires_dom_idx(
	mut commands: Commands,
	attributes: Query<(Entity, &Attributes)>,
	dyn_attrs: Query<(), (With<AttributeOf>, Added<SignalEffect>)>,
	dyn_text_nodes: Query<
		Entity,
		(With<TextNode>, With<SignalEffect>, Without<AttributeOf>),
	>,
	dyn_fragments: Query<Entity, (With<FragmentNode>, Added<SignalEffect>)>,
	parents: Query<&ChildOf>,
	elements: Query<Entity, With<ElementNode>>,
) -> Result {
	// 1. fragments
	for entity in dyn_fragments.iter() {
		let parent = parents
			.iter_ancestors(entity)
			.find(|e| elements.contains(*e))
			.ok_or_else(|| {
				bevyhow!(
					"FragmentNode with SignalEffect must have an ElementNode parent"
				)
			})?;
		// fragment nodes do not need an idx, but their parents do
		commands.entity(parent).insert(RequiresDomIdx);
	}

	// 2. text nodes
	for entity in dyn_text_nodes.iter() {
		let parent = parents
			.iter_ancestors(entity)
			.find(|e| elements.contains(*e))
			.ok_or_else(|| {
				bevyhow!(
					"TextNode with SignalEffect must have an ElementNode parent"
				)
			})?;
		// text node also needs idx for creating the boundary comment nodes
		commands.entity(entity).insert(RequiresDomIdx);
		commands.entity(parent).insert(RequiresDomIdx);
	}
	// 3. attributes
	for (entity, _) in attributes
		.iter()
		.filter(|(_, attrs)| attrs.iter().any(|attr| dyn_attrs.contains(attr)))
	{
		commands.entity(entity).insert(RequiresDomIdx);
	}
	Ok(())
}


/// Recursively applies a [`DomIdx`] to root nodes spawned *without* one,
/// not counting roots that are spawned with one like client islands.
#[allow(dead_code)]
pub(super) fn apply_root_dom_idx(
	mut commands: Commands,
	html_constants: Res<HtmlConstants>,
	roots: Populated<Entity, Added<HtmlDocument>>,
	children: Query<&Children>,
	requires_idx: Query<(), Added<RequiresDomIdx>>,
) {
	let mut id = 0;

	// find only the top level roots
	for root in roots.iter() {
		for entity in children
			//dfs allows for client islands to accurately pick up the next index
			.iter_descendants_depth_first(root)
			.filter(|entity| requires_idx.contains(*entity))
		{
			commands
				.entity(entity)
				.remove::<RequiresDomIdx>()
				.insert(DomIdx::new(id));

			commands.spawn((
				AttributeOf::new(entity),
				AttributeKey::new(html_constants.dom_idx_key.clone()),
				TextNode::new(id.to_string()),
			));
			id += 1;
		}
	}
}

/// Recursively applies a [`DomIdx`] to children of root nodes spawned *with* one,
/// like client islands.
#[allow(dead_code)]
pub(super) fn apply_client_island_dom_idx(
	mut commands: Commands,
	html_constants: Res<HtmlConstants>,
	// definition of a root: any fragment or element without a parent
	roots: Populated<
		(Entity, &DomIdx),
		(Added<DomIdx>, Without<ChildOf>, Without<AttributeOf>),
	>,
	children: Query<&Children>,
	requires_idx: Query<(), Added<RequiresDomIdx>>,
) {
	for (root, idx) in roots.iter() {
		let mut id = idx.inner() + 1; // start at the next index after the root
		for entity in children
			//dfs exclusive, root already has a DomIdx
			.iter_descendants_depth_first(root)
			.filter(|entity| requires_idx.contains(*entity))
		{
			commands
				.entity(entity)
				.remove::<RequiresDomIdx>()
				.insert(DomIdx::new(id));
			commands.spawn((
				AttributeOf::new(entity),
				AttributeKey::new(html_constants.dom_idx_key.clone()),
				TextNode::new(id.to_string()),
			));
			id += 1;
		}
	}
}


// see render_html.rs for more tests
#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn applies_ids() {
		let mut app = App::new();
		app.add_plugins((ApplySnippetsPlugin, SignalsPlugin));
		let world = app.world_mut();
		world.init_resource::<HtmlConstants>();
		let (get, _set) = signal(2);
		let div = world
			.spawn((HtmlDocument, rsx! {
				<div onclick=||{}>
					"child 1"
					<span>"child with signal"{get}</span>
					"child 2"
				</div>
			}))
			.get::<Children>()
			.unwrap()[0];
		world.run_schedule(ApplySnippets);
		world.run_system_once(super::apply_root_dom_idx).unwrap();

		world.get::<DomIdx>(div).unwrap().xpect().to_be(&DomIdx(0));
	}
}

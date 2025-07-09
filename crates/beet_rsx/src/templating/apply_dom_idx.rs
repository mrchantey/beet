use crate::prelude::*;
use beet_core::as_beet::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;



/// Only [`ElementNodes`](ElementNode) matching one of the below filters
/// require a [`DomIdx`]:
/// - [`EventTarget`] for event binding
/// - [`TextNodeParent`] for splitting child text nodes
/// - [`ClientOnlyDirective`] for client islands
/// - [`ClientLoadDirective`] for client islands
/// - [`SignalReceiver`] attributes for updating element attributes (properties)
#[derive(SystemParam)]
pub struct RequiresIdx<'w, 's> {
	requires_dom_idx_attr: Query<
		'w,
		's,
		Entity,
		Or<(
			Added<EventTarget>,
			Added<TextNodeParent>,
			Added<ClientOnlyDirective>,
			Added<ClientLoadDirective>,
		)>,
	>,
	attributes: Query<'w, 's, &'static Attributes>,
	dyn_attrs:
		Query<'w, 's, (), (With<AttributeOf>, Added<SignalReceiver<String>>)>,
}
impl RequiresIdx<'_, '_> {
	pub fn requires(&self, entity: Entity) -> bool {
		self.requires_dom_idx_attr.contains(entity)
			|| self
				.attributes
				.get(entity)
				.map(|attrs| {
					attrs.iter().any(|attr| self.dyn_attrs.contains(attr))
				})
				.unwrap_or(false)
	}
}
/// Recursively applies a [`DomIdx`] to root nodes spawned *without* one,
/// not counting roots that are spawned with one like client islands.
#[allow(dead_code)]
pub(super) fn apply_root_dom_idx(
	mut commands: Commands,
	html_constants: Res<HtmlConstants>,
	roots: Populated<Entity, Added<HtmlDocument>>,
	children: Query<&Children>,
	requires_idx: RequiresIdx,
) {
	let mut id = 0;

	// find only the top level roots
	for root in roots.iter() {
		for entity in children
			//dfs allows for client islands to accurately pick up the next index
			.iter_descendants_depth_first(root)
			.filter(|entity| requires_idx.requires(*entity))
		{
			// only 'dynamic' elements need a DomIdx
			commands.entity(entity).insert(DomIdx::new(id));

			commands.spawn((
				AttributeOf::new(entity),
				AttributeKey::new(html_constants.dom_idx_key.clone()),
				AttributeLit::new(id.to_string()),
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
	roots: Populated<(Entity, &DomIdx), (Added<DomIdx>, Without<ChildOf>)>,
	children: Query<&Children>,
	requires_idx: RequiresIdx,
) {
	for (root, idx) in roots.iter() {
		let mut id = idx.inner() + 1; // start at the next index after the root
		for entity in children
			//dfs exclusive, root already has a DomIdx
			.iter_descendants_depth_first(root)
			.filter(|entity| requires_idx.requires(*entity))
		{
			commands.entity(entity).insert(DomIdx::new(id));
			commands.spawn((
				AttributeOf::new(entity),
				AttributeKey::new(html_constants.dom_idx_key.clone()),
				AttributeLit::new(id.to_string()),
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
		let mut world = World::new();
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
		world
			.run_system_once(apply_snippets_to_instances)
			.unwrap()
			.unwrap();
		world
			.run_system_once(super::super::apply_text_node_parents)
			.unwrap();
		world.run_system_once(super::apply_root_dom_idx).unwrap();

		world.get::<DomIdx>(div).unwrap().xpect().to_be(&DomIdx(0));

		let children = world.get::<Children>(div).unwrap();
		world
			.get::<DomIdx>(children[1])
			.unwrap()
			.xpect()
			.to_be(&DomIdx(1));
	}
}

use crate::prelude::*;
use beet_bevy::prelude::HierarchyQueryExtExt;
use beet_common::as_beet::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// Only apply [`TreeIdx`] to nodes that require it. This restriction allows
/// for template reloading, all other nodes can move around and the ids remain
/// the same.
/// Only [`ElementNodes`](ElementNode) matching one of the below filters
/// require a [`TreeIdx`]
#[derive(SystemParam)]
pub struct RequiresIdx<'w, 's> {
	requires_tree_idx_attr: Query<
		'w,
		's,
		Entity,
		Or<(
			Added<EventKey>,
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
		self.requires_tree_idx_attr.contains(entity)
			|| self
				.attributes
				.get(entity)
				.map(|attrs| {
					attrs.iter().any(|attr| self.dyn_attrs.contains(attr))
				})
				.unwrap_or(false)
	}
}
/// Recursively applies a [`TreeIdx`] to root nodes spawned *without* one,
/// not counting roots that are spawned with one like client islands.
pub(super) fn apply_root_tree_idx(
	mut commands: Commands,
	html_constants: Res<HtmlConstants>,
	// definition of a root: any fragment or element without a parent
	roots: Populated<
		Entity,
		(
			Without<ChildOf>,
			Without<TreeIdx>,
			// just client directives?
			Or<(Added<FragmentNode>, Added<ElementNode>, Added<TemplateNode>)>,
		),
	>,
	children: Query<&Children>,
	requires_idx: RequiresIdx,
) {
	let mut id = 0;
	// even though we're iterating roots theres usually only one entrypoint, 
	// ie a BundleRoute, but it should still work with multiple
	for root in roots.iter() {
		for entity in children
			//bfs
			.iter_descendants_inclusive(root)
			.filter(|entity| requires_idx.requires(*entity))
		{
			// only 'dynamic' elements need a TreeIdx
			commands.entity(entity).insert(TreeIdx::new(id));
			println!("Applying TreeIdx {} to {}", id, entity);

			commands.spawn((
				AttributeOf::new(entity),
				AttributeKey::new(html_constants.tree_idx_key.clone()),
				AttributeLit::new(id.to_string()),
			));
			id += 1;
		}
	}
}

/// Recursively applies a [`TreeIdx`] to children of root nodes spawned *with* one,
/// like client islands.
pub(super) fn apply_child_tree_idx(
	mut commands: Commands,
	html_constants: Res<HtmlConstants>,
	// definition of a root: any fragment or element without a parent
	roots: Populated<
		(Entity, &TreeIdx),
		(
			Without<ChildOf>,
			Or<(Added<FragmentNode>, Added<ElementNode>, Added<TemplateNode>)>,
		),
	>,
	children: Query<&Children>,
	requires_idx: RequiresIdx,
) {
	for (root, idx) in roots.iter() {
		let mut id = idx.inner();
		for entity in children
			//bfs
			.iter_descendants_inclusive(root)
			.filter(|entity| requires_idx.requires(*entity))
		{
			// only 'dynamic' elements need a TreeIdx
			commands.entity(entity).insert(TreeIdx::new(id));

			commands.spawn((
				AttributeOf::new(entity),
				AttributeKey::new(html_constants.tree_idx_key.clone()),
				AttributeLit::new(id.to_string()),
			));
			id += 1;
		}
	}
}

/// Similar to an [`Entity`], contaning a unique identifier for this node in
/// a templating tree.
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

impl std::fmt::Display for TreeIdx {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "TreeIdx({})", self.0)
	}
}

impl TreeIdx {
	pub fn new(idx: u32) -> Self { Self(idx) }
	pub fn inner(&self) -> u32 { self.0 }
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
		let entity = world
			.spawn(rsx! {
				<div onclick=||{}>
					"child 1"
					<span>"child with signal"{get}</span>
					"child 2"
				</div>
			})
			.id();
		world
			.run_system_once(super::super::apply_text_node_parents)
			.unwrap();
		world.run_system_once(super::apply_root_tree_idx).unwrap();

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

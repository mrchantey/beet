use beet_core::prelude::*;
use bevy::prelude::*;


/// A node which is a descendant of a template root
#[derive(Debug, Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = TemplateChildren)]
pub struct TemplateChildOf(Entity);

/// Added to the root of a template, pointing to all nodes which are
/// descendants of the template root, excluding other templates.
#[derive(Debug, Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = TemplateChildOf, linked_spawn)]
pub struct TemplateChildren(Vec<Entity>);


/// Creates a [`TemplateNodes`] relation for each template root,
/// pointing to every non-template node which is a descendant.
pub fn apply_template_children(
	mut commands: Commands,
	template_roots: Populated<
		Entity,
		(
			Added<InstanceRoot>,
			Or<(
				// document root, also considered a template
				Without<ChildOf>,
				// template
				With<TemplateOf>,
			)>,
		),
	>,
	children: Query<&Children>,
	// node_tags: Query<&NodeTag>,
) {
	for root in template_roots.iter() {
		// TODO self-relations, iter_descendants_inclusive
		for child in children.iter_descendants(root) {
			// let name = node_tags
			// 	.get(child)
			// 	.map(|tag| tag.0.as_str())
			// 	.unwrap_or("unknown");
			commands.entity(child).insert(TemplateChildOf(root));
		}
	}
}




#[cfg(test)]
mod test {
	use super::*;
	use crate::apply_snippets::flush_on_spawn_deferred_recursive;
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[template]
	pub fn MyTemplate() -> impl Bundle {
		rsx! {<div>hello world!</div> }
	}


	#[test]
	fn works_no_children() {
		let mut world = World::new();
		let root = world
			.spawn(rsx! {
				<div/>
			})
			.id();
		world.run_system_cached(apply_template_children).unwrap();
		world
			.entity(root)
			.get::<TemplateChildren>()
			.unwrap()
			.len()
			.xpect()
			.to_be(1);
	}
	#[test]
	fn works() {
		let mut world = World::new();
		let root = world
			.spawn(rsx! {
				<div>
					<MyTemplate/>
					<MyTemplate/>
				</div>
			})
			.id();
		world
			.run_system_cached_with(flush_on_spawn_deferred_recursive, root)
			.unwrap()
			.unwrap();
		world.run_system_cached(apply_template_children).unwrap();

		let nodes = world.entity(root).get::<TemplateChildren>().unwrap();
		nodes.len().xpect().to_be(3);

		world
			.query_once::<&TemplateChildren>()
			.len()
			.xpect()
			.to_be(3);
	}
}

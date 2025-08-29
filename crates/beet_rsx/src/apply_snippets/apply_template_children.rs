use beet_core::prelude::*;
use bevy::prelude::*;

use crate::prelude::HtmlDocument;


/// A node which is a descendant of a template root
#[derive(Debug, Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = TemplateChildren)]
pub struct TemplateChildOf(Entity);

/// Added to the root of a template, pointing to all nodes which are
/// descendants of the template root, excluding other templates.
#[derive(Debug, Clone, Deref, Reflect, Component)]
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
				// instance without parent is a root
				Without<ChildOf>,
				// documents are roots
				With<HtmlDocument>,
				// templates are roots
				With<TemplateOf>,
			)>,
		),
	>,
	children: Query<&Children>,
	// node_tags: Query<&NodeTag>,
) {
	for root in template_roots.iter() {
		for child in children.iter_descendants(root) {
			commands.entity(child).insert(TemplateChildOf(root));
		}
	}
}




#[cfg(test)]
mod test {
	use super::*;
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[template]
	pub fn MyTemplate() -> impl Bundle {
		rsx! { <div>hello world!</div> }
	}


	#[test]
	fn works_no_children() {
		let mut world = World::new();
		let root = world
			.spawn(rsx! { <div /> })
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
					<MyTemplate>
						<span>
					</MyTemplate>
					<MyTemplate/>
				</div>
			})
			.id();
		world.run_system_cached(OnSpawnDeferred::flush).unwrap();
		world.run_system_cached(apply_template_children).unwrap();

		world
			.entity(root)
			.get::<TemplateChildren>()
			.unwrap()
			.len()
			.xpect()
			.to_be(4); // div, mytemplate, span, mytemplate

		world
			.query_once::<&TemplateChildren>()
			.len()
			.xpect()
			.to_be(3);
	}
}

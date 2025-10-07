use crate::prelude::HtmlDocument;
use beet_core::prelude::*;
use beet_dom::prelude::*;


/// Marker type indicating this entity was spawned via [`bundle_endpoint`].
#[derive(Component)]
pub struct HandlerBundle;

/// A node which is a descendant of a template root
#[derive(Debug, Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = TemplateChildren)]
pub struct TemplateChildOf(Entity);

/// Added to the root of a template, pointing to all nodes which are
/// children of the template root, excluding other [`TemplateRoot`]s
/// which have not yet been resolved to children by apply_slots.
#[derive(Debug, Clone, Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = TemplateChildOf, linked_spawn)]
pub struct TemplateChildren(Vec<Entity>);


/// Creates a [`TemplateChildren`] relation for each template root,
/// pointing to every child which is a descendant.
pub fn apply_template_children(
	In(entity): In<Entity>,
	mut commands: Commands,
	template_roots: Query<
		(),
			Or<(
				// instance without parent is a root
				Without<ChildOf>,
				// documents are roots, even if they have a parent
				With<HtmlDocument>,
				// templates are roots
				With<TemplateOf>,
				// handler bundles are roots
				With<HandlerBundle>,
			)>,
	>,
	children: Query<&Children>,
	// these appear in Children if resolved by blocks instead of tags:
	// - like this 		`rsx!{<div>{rsx!{<MyTemplate/>}}</div>}`
	// - instead of 	`rsx!{<div><MyTemplate/></div>}`
	template_nodes: Query<(), (With<TemplateNode>, Without<TemplateRoot>)>,
) {
	if template_roots.contains(entity) {
		let mut stack = vec![entity];
		// recurse into all children, but stop if template_nodes.contains,
		// 	 to children by apply_slots
		while let Some(current) = stack.pop() {
			if let Ok(children) = children.get(current) {
				for child in children.iter() {
					if !template_nodes.contains(child) {
						commands.entity(child).insert(TemplateChildOf(entity));
						stack.push(child);
					}
				}
			}
		}
	}
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[template]
	pub fn MyTemplate() -> impl Bundle {
		rsx! { <div><div><slot/>hello world!</div></div> }
	}


	#[test]
	fn no_children() {
		World::new()
			.spawn(rsx! { <div /> })
			.get::<TemplateChildren>()
			.unwrap()
			.len()
			.xpect_eq(1);
	}
	#[test]
	fn visits_nested_rsx() {
		World::new()
			.spawn(rsx! { <div>{rsx!{<div/>}}</div> })
			.get::<TemplateChildren>()
			.unwrap()
			.len()
			.xpect_eq(3); // div, BlockNode/SnippetRoot, div
	}
	#[test]
	fn skips_template_roots_simple() {
		let mut world = World::new();
		world
			.spawn(rsx! {
				<div>
					<MyTemplate/>
				</div>
			})
			.get::<TemplateChildren>()
			.unwrap()
			.len()
			.xpect_eq(2); // div, mytemplate

		world.query_once::<&TemplateChildren>().len().xpect_eq(2);
	}
	#[test]
	fn skips_template_roots_complex() {
		let mut world = World::new();
		world
			.spawn(rsx! {
				<div>
					<MyTemplate>
						<span>
					</MyTemplate>
					<MyTemplate/>
				</div>
			})
			.get::<TemplateChildren>()
			.unwrap()
			.len()
			.xpect_eq(4); // div, mytemplate, span, mytemplate

		world.query_once::<&TemplateChildren>().len().xpect_eq(3);
	}
	#[test]
	fn skips_resolved_template() {
		let mut world = World::new();
		world
			.spawn(rsx! { <div>{rsx!{<MyTemplate/>}}</div> })
			.get::<TemplateChildren>()
			.unwrap()
			.len()
			.xpect_eq(2); // div, BlockNode, not MyTemplate inner

		world.query_once::<&TemplateChildren>().len().xpect_eq(2);
	}
}

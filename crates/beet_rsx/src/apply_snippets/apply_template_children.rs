use crate::prelude::HtmlDocument;
use beet_core::prelude::*;
use beet_dom::prelude::*;


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
) {
	for root in template_roots.iter() {
		for child in children.iter_descendants(root) {
			commands.entity(child).insert(TemplateChildOf(root));
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
	fn works_no_children() {
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
	fn skips_resolved_template() {
		World::new()
			.spawn(rsx! { <div>{rsx!{<MyTemplate/>}}</div> })
			.get::<TemplateChildren>()
			.unwrap()
			.len()
			.xpect_eq(3); // div, BlockNode/SnippetRoot, MyTemplate
	}
	#[test]
	fn works() {
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
}

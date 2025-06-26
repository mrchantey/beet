use super::*;
use beet_bevy::prelude::HierarchyQueryExtExt;
use beet_common::prelude::*;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;




/// Apply any [`StyleId`] in a template to any  Runs before apply slots
pub fn apply_style_id_attributes(
	html_constants: Res<HtmlConstants>,
	mut commands: Commands,
	parents: Query<&ChildOf>,
	children: Query<&Children>,
	elements: Query<(), With<ElementNode>>,
	templates: Query<&TemplateRoot, With<StyleCascade>>,
	// exclude single NodePortalTarget entities
	query: Populated<(Entity, &StyleId), Without<NodePortalTarget>>,
) {
	let mut visited = HashSet::new();

	let mut apply_to_root = |root: Entity, styleid: StyleId| {
		for child in children
			.iter_descendants_inclusive(root)
			.filter(|en| elements.contains(*en))
		{
			if visited.contains(&(child, styleid)) {
				continue;
			}
			commands.spawn((
				AttributeOf::new(child),
				AttributeKey::new(html_constants.style_id_attribute(styleid)),
			));
			visited.insert((child, styleid));
		}
	};

	for (entity, styleid) in query.iter() {
		let root = parents.root_ancestor(entity);
		apply_to_root(root, *styleid);
		// also apply to StyleCascade templates
		for child in children
			.iter_descendants_inclusive(root)
			.filter_map(|en| templates.get(en).ok())
		{
			apply_to_root(**child, *styleid);
		}
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use crate::templating::apply_slots;
	use beet_common::node::HtmlConstants;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;


	fn parse(bundle: impl Bundle) -> String {
		let mut world = World::new();
		world.init_resource::<HtmlConstants>();
		let entity = world.spawn(bundle).id();
		world
			.run_system_once(spawn_templates)
			.unwrap()
			.unwrap();
		world
			.run_system_once(super::apply_style_id_attributes)
			.unwrap();
		world.run_system_once(apply_slots).ok();
		world.run_system_once_with(render_fragment, entity).unwrap()
	}

	#[template]
	fn MyTemplate() -> impl Bundle {
		rsx! {<div/><slot/>}
	}

	#[test]
	fn assigns_id_attr() {
		parse(rsx! {<style {StyleId::new(0)}/>})
			.xpect()
			.to_be_str("<style data-beet-style-id-0/>");
	}
	#[test]
	fn deduplicates() {
		parse(rsx! {
			<div>
			<style {StyleId::new(0)}/>
			<style {StyleId::new(0)}/>
			</div>
		})
		.xpect()
		.to_be_str("<div data-beet-style-id-0><style data-beet-style-id-0/><style data-beet-style-id-0/></div>");
	}
	#[test]
	fn assigns_id_to_all() {
		parse(rsx! {
			<div>
			<style {StyleId::new(0)}/>
			<span/>
			</div>
		})
		.xpect()
		.to_be_str("<div data-beet-style-id-0><style data-beet-style-id-0/><span data-beet-style-id-0/></div>");
	}
	#[test]
	fn ignores_templates() {
		parse(rsx! {
			<style {StyleId::new(0)}/>
			<MyTemplate/>
		})
		.xpect()
		.to_be_str("<style data-beet-style-id-0/><div/>");
	}
	#[test]
	fn applies_to_slots() {
		parse(rsx! {
			<style {StyleId::new(0)}/>
			<MyTemplate>
				<span/>
			</MyTemplate>
		})
		.xpect()
		.to_be_str(
			"<style data-beet-style-id-0/><div/><span data-beet-style-id-0/>",
		);
	}
	#[test]
	fn cascades() {
		parse(rsx! {
			<style {StyleId::new(0)}/>
			<MyTemplate style:cascade/>
		})
		.xpect()
		.to_be_str("<style data-beet-style-id-0/><div data-beet-style-id-0/>");
	}
}

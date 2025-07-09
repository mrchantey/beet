use super::*;
use beet_core::prelude::HierarchyQueryExtExt;
use beet_common::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

/// Apply an [`AttributeKey`] with corresponding [`StyleId`] to elements with one.
/// runs before [`apply_slots`] so that slot children are treated as belonging
/// to the parent template.
///
/// ## Example
/// In the below example, `div`, `span` and the contents of `Template2` will
/// have the style applied, and the contents of `Template1` will not.
/// ```rust ignore
///
/// <div>
/// 	<Template1>
///   	<span/>
///   </Template1>
///   </Template2 style:cascade>
/// </div>
/// <style>
/// ...
/// </style>
/// ```
pub(super) fn apply_style_id_attributes(
	// visit all docment roots and templates that aren't yet children
	roots: Populated<Entity, Or<(Added<HtmlDocument>, Added<TemplateOf>)>>,
	children: Query<&Children>,
	style_ids: Query<&StyleId>,
	mut builder: ApplyAttributes,
) {
	for entity in roots.iter() {
		let mut visited = HashSet::<(Entity, StyleId)>::default();
		for styleid in children
			.iter_descendants(entity)
			.filter_map(|en| style_ids.get(en).ok())
		{
			builder.apply_recursive(&mut visited, entity, *styleid);
		}
	}
}

#[derive(SystemParam)]
pub(super) struct ApplyAttributes<'w, 's> {
	html_constants: Res<'w, HtmlConstants>,
	commands: Commands<'w, 's>,
	children: Query<'w, 's, &'static Children>,
	elements: Query<'w, 's, &'static NodeTag, With<ElementNode>>,
	cascade_templates: Query<'w, 's, &'static TemplateRoot, With<StyleCascade>>,
}

impl ApplyAttributes<'_, '_> {
	fn apply_recursive(
		&mut self,
		visited: &mut HashSet<(Entity, StyleId)>,
		entity: Entity,
		styleid: StyleId,
	) {
		if visited.contains(&(entity, styleid)) {
			return;
		}
		visited.insert((entity, styleid));
		if let Ok(tag) = self.elements.get(entity)
			&& !self.html_constants.hoist_to_head_tags.contains(&tag.0)
		{
			self.commands.spawn((
				AttributeOf::new(entity),
				AttributeKey::new(
					self.html_constants.style_id_attribute(styleid),
				),
			));
		}
		for template in self
			.cascade_templates
			.iter_direct_descendants(entity)
			.collect::<Vec<_>>()
		{
			self.apply_recursive(visited, template, styleid);
		}

		for child in self
			.children
			.iter_direct_descendants(entity)
			.collect::<Vec<_>>()
		{
			self.apply_recursive(visited, child, styleid);
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
		let entity = world.spawn((HtmlDocument, bundle)).id();
		world
			.run_system_once(apply_snippets_to_instances)
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
		parse(rsx! {<style {StyleId::new(0)}/><span/>})
			.xpect()
			.to_be_str("<style/><span data-beet-style-id-0/>");
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
		.to_be_str("<div data-beet-style-id-0><style/><style/></div>");
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
		.to_be_str(
			"<div data-beet-style-id-0><style/><span data-beet-style-id-0/></div>",
		);
	}
	#[test]
	fn ignores_templates() {
		parse(rsx! {
			<style {StyleId::new(0)}/>
			<MyTemplate/>
		})
		.xpect()
		.to_be_str("<style/><div/>");
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
		.to_be_str("<style/><div/><span data-beet-style-id-0/>");
	}
	#[test]
	fn cascades() {
		parse(rsx! {
			<style {StyleId::new(0)}/>
			<MyTemplate style:cascade/>
		})
		.xpect()
		.to_be_str("<style/><div data-beet-style-id-0/>");
	}
	#[test]
	fn nested_template() {
		#[template]
		fn StyledTemplate() -> impl Bundle {
			rsx! {
				<style {StyleId::new(0)}/>
				<div>
			}
		}

		parse(rsx! {
			<StyledTemplate/>
		})
		.xpect()
		.to_be_str("<style/><div data-beet-style-id-0/>");
	}
}

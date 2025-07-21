use super::*;
use beet_core::prelude::HierarchyQueryExtExt;
use beet_core::prelude::*;
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
	style_ids: Query<&LangSnippetHash, With<StyleElement>>,
	mut builder: ApplyAttributes,
) {
	for root in roots.iter() {
		let mut visited = HashSet::<(Entity, LangSnippetHash)>::default();
		for styleid in children
			.iter_descendants(root)
			.filter_map(|en| style_ids.get(en).ok())
		{
			builder.apply_recursive(&mut visited, root, *styleid);
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
		visited: &mut HashSet<(Entity, LangSnippetHash)>,
		entity: Entity,
		styleid: LangSnippetHash,
	) {
		if visited.contains(&(entity, styleid)) {
			return;
		}
		visited.insert((entity, styleid));
		if let Ok(tag) = self.elements.get(entity)
			&& !self.html_constants.ignore_style_id_tags.contains(&tag.0)
		{
			self.commands.spawn((
				AttributeOf::new(entity),
				// hash will be converted to attribute key in compress_style_ids.rs
				styleid.clone(),
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
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[template]
	fn MyTemplate() -> impl Bundle {
		rsx! {<div/><slot/>}
	}

	#[test]
	fn assigns_id_attr() {
		HtmlDocument::parse_bundle(
			rsx! {<style {LangSnippetHash::new(0)}/><span/>},
		)
		.xpect()
		.to_be_snapshot();
	}
	#[test]
	fn deduplicates() {
		HtmlDocument::parse_bundle(rsx! {
			<div>
			<style {LangSnippetHash::new(0)}/>
			<style {LangSnippetHash::new(0)}/>
			</div>
		})
		.xpect()
		.to_be_snapshot();
	}
	#[test]
	fn assigns_id_to_all() {
		HtmlDocument::parse_bundle(rsx! {
			<div>
			<style {LangSnippetHash::new(0)}/>
			<span/>
			</div>
		})
		.xpect()
		.to_be_snapshot();
	}
	#[test]
	fn ignores_templates() {
		HtmlDocument::parse_bundle(rsx! {
			<style {LangSnippetHash::new(0)}/>
			<MyTemplate/>
		})
		.xpect()
		.to_be_snapshot();
	}
	#[test]
	fn applies_to_slots() {
		HtmlDocument::parse_bundle(rsx! {
			<style {LangSnippetHash::new(0)}/>
			<MyTemplate>
				<span/>
			</MyTemplate>
		})
		.xpect()
		.to_be_snapshot();
	}
	#[test]
	fn cascades() {
		HtmlDocument::parse_bundle(rsx! {
			<style {LangSnippetHash::new(0)}/>
			<MyTemplate style:cascade/>
		})
		.xpect()
		.to_be_snapshot();
	}
	#[test]
	fn nested_template() {
		#[template]
		fn StyledTemplate() -> impl Bundle {
			rsx! {
				<style {LangSnippetHash::new(0)}/>
				<div>
			}
		}

		HtmlDocument::parse_bundle(rsx! {
			<StyledTemplate/>
		})
		.xpect()
		.to_be_snapshot();
	}

	#[test]
	#[cfg(feature = "css")]
	fn style_template() {
		use crate::as_beet::*;

		#[template]
		fn Style() -> impl Bundle {
			rsx_combinator! {r"
<div>pow</div>
<style>
	body { color: red; }
</style>
			"}
		}

		HtmlDocument::parse_bundle(rsx! {<Style/>})
			.xpect()
			.to_be_snapshot();
	}
}

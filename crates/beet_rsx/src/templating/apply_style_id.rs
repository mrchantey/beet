use super::*;
use crate::prelude::*;
use beet_core::prelude::HierarchyQueryExtExt;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::platform::collections::HashMap;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

/// Apply an [`AttributeKey`] with corresponding [`StyleId`] to elements with one
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
pub fn apply_style_id(
	// visit all docment roots and templates that aren't yet children
	roots: Populated<Entity, Added<HtmlDocument>>,
	children: Query<&Children>,
			// TODO self-relations, just template_children
	template_children: Query<&TemplateChildren>,
	style_ids: Query<&LangSnippetHash, With<StyleElement>>,
	mut builder: ApplyAttributes,
) {
	for root in roots.iter() {
		let mut index_incr = 0;
		let mut index_map = HashMap::<LangSnippetHash, u64>::default();
		// ensure we only apply each style id to an entity once
		let mut visited = HashSet::<(Entity, u64)>::default();
		let mut get_next_index = |hash: &LangSnippetHash| {
			let index = index_map.entry(hash.clone()).or_insert_with(|| {
				let idx = index_incr;
				index_incr += 1;
				idx
			});
			*index
		};
		for template_children in children
			.iter_descendants_inclusive(root)
			.filter_map(|en| template_children.get(en).ok())
		{
			for styleid in template_children
				.iter()
				.filter_map(|en| style_ids.get(en).ok())
			{
				let index = get_next_index(styleid);
				builder.apply_recursive(&mut visited, template_children, index);
			}
		}
	}
}

/// Recursively apply attributes, only visiting children if either:
/// 1. They are not a [`TemplateNode`]
/// 2. They are a [`TemplateNode`] with a [`StyleCascade`]
#[derive(SystemParam)]
pub struct ApplyAttributes<'w, 's> {
	constants: Res<'w, HtmlConstants>,
	commands: Commands<'w, 's>,
	elements: Query<'w, 's, &'static NodeTag, With<ElementNode>>,
	lang_elements:
		Query<'w, 's, (&'static LangSnippetHash, &'static mut InnerText)>,
	// children: Query<'w, 's, &'static Children>,
	// cascade: Query<'w, 's, &'static Children, With<StyleCascade>>,
	// cascade_children: Query<
	// 	'w,
	// 	's,
	// 	&'static Children,
	// 	Or<(
	// 		Without<TemplateNode>,
	// 		(With<TemplateNode>, With<StyleCascade>),
	// 	)>,
	// >,
	// original_templates: Query<'w, 's, &'static OriginalTemplateChildren>,
}

impl ApplyAttributes<'_, '_> {
	fn apply_recursive(
		&mut self,
		visited: &mut HashSet<(Entity, u64)>,
		entities: &TemplateChildren,
		styleid: u64,
	) {
		for entity in entities.iter() {
			self.apply_to_entity(visited, entity, styleid);
		}
	}
	fn apply_to_entity(
		&mut self,
		visited: &mut HashSet<(Entity, u64)>,
		entity: Entity,
		styleid: u64,
	) {
		if visited.contains(&(entity, styleid)) {
			return;
		}
		visited.insert((entity, styleid));
		// parse_lightning uses the hash for scoped selectors, replace it
		// with an index
		if let Ok((hash, mut text)) = self.lang_elements.get_mut(entity) {
			let original_id = self.constants.style_id_attribute(**hash);
			let new_id = self.constants.style_id_attribute(styleid);
			text.0 = text.0.replace(&original_id, &new_id);
		};

		if let Ok(tag) = self.elements.get(entity)
			&& !self.constants.ignore_style_id_tags.contains(&tag.0)
		{
			self.commands.spawn((
				AttributeOf::new(entity),
				AttributeKey::new(self.constants.style_id_attribute(styleid)),
			));
		}
		// for cascade_children in self
		// 	.children
		// 	.iter_direct_descendants(entity)
		// 	.filter_map(|en| self.cascade.get(en).ok())
		// {
		// 	for cascade_children in cascade_children
		// 		.iter()
		// 		.filter_map(|en| self.original_templates.get(en).ok())
		// 	{
		// 		self.apply_recursive(visited, cascade_children, styleid);
		// 	}

		// }
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

	fn replace_hash() -> impl Bundle {
		OnSpawn::new(move |entity| {
			entity.insert(LangSnippetHash::new(0));
		})
	}

	#[test]
	fn assigns_id_attr() {
		HtmlDocument::parse_bundle(rsx! {<style {replace_hash()}/><span/>})
			.xpect()
			.to_be_snapshot();
	}

	#[test]
	#[cfg(feature = "css")]
	fn updates_style_content() {
		HtmlDocument::parse_bundle(rsx! {
			<style> body{ color: red; }</style>
			<div/>
		})
		.xpect()
		.to_be_snapshot();
	}
	#[test]
	fn deduplicates() {
		HtmlDocument::parse_bundle(rsx! {
			<div>
			<style {replace_hash()}/>
			<style {replace_hash()}/>
			</div>
		})
		.xpect()
		.to_be_snapshot();
	}
	#[test]
	fn assigns_id_to_all() {
		HtmlDocument::parse_bundle(rsx! {
			<div>
			<style {replace_hash()}/>
			<span/>
			</div>
		})
		.xpect()
		.to_be_snapshot();
	}
	#[test]
	fn ignores_templates() {
		HtmlDocument::parse_bundle(rsx! {
			<style {replace_hash()}/>
			<MyTemplate/>
		})
		.xpect()
		.to_be_snapshot();
	}
	// #[test]
	// fn visits_templates() {
	// 	HtmlDocument::parse_bundle(rsx! {
	// 		<style {replace_hash()}/>
	// 		<MyTemplate/>
	// 		<MyTemplate/>
	// 	})
	// 	.xpect()
	// 	.to_be_snapshot();
	// }
	#[test]
	fn applies_to_slots() {
		HtmlDocument::parse_bundle(rsx! {
			<style {replace_hash()}/>
			<MyTemplate>
				<span/>
			</MyTemplate>
		})
		.xpect()
		.to_be_snapshot();
	}
	#[test]
	#[ignore = "todo remove templateof"]
	fn cascades() {
		HtmlDocument::parse_bundle(rsx! {
			<style {replace_hash()}/>
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
				<style {replace_hash()}/>
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

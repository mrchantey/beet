use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;

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
	lang_elements: Query<'w, 's, &'static mut InnerText, With<StyleElement>>,
	cascade: Query<'w, 's, &'static Children, With<StyleCascade>>,
	template_children: Query<'w, 's, &'static TemplateChildren>,
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
		// replace parse_lightning selector hashes with an index
		if let Ok(mut text) = self.lang_elements.get_mut(entity) {
			let original_id = self.constants.style_id_attribute_placeholder();
			let new_id = self.constants.style_id_attribute(styleid);
			text.0 = text.0.replace(&original_id, &new_id);
		};

		// add the attribute to allowed elements
		if let Ok(tag) = self.elements.get(entity)
			&& !self.constants.ignore_style_id_tags.contains(&tag.0)
		{
			self.commands.spawn((
				AttributeOf::new(entity),
				AttributeKey::new(self.constants.style_id_attribute(styleid)),
			));
		}

		// apply to templates marked with style:cascade
		if let Ok(cascade) = self.cascade.get(entity) {
			for template_children in cascade
				.iter()
				.filter_map(|en| {
					self.template_children
						.get(en)
						.map(|children| children.clone())
						.ok()
				})
				.collect::<Vec<_>>()
			{
				self.apply_recursive(visited, &template_children, styleid);
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[template]
	fn MyTemplate() -> impl Bundle {
		rsx! {
			<div />
			<slot />
		}
	}

	fn replace_hash() -> impl Bundle {
		OnSpawnTyped::new(move |entity| {
			entity.insert(LangSnippetHash::new(0));
		})
	}

	#[test]
	fn assigns_id_attr() {
		HtmlDocument::parse_bundle(rsx! {
			<style {replace_hash()} />
			<span />
		})
		.xpect_snapshot();
	}

	#[test]
	#[cfg(feature = "css")]
	fn updates_style_content() {
		HtmlDocument::parse_bundle(rsx! {
			<style> body{ color: red; }</style>
			<div/>
		})
		.xpect_snapshot();
	}
	#[test]
	fn deduplicates() {
		HtmlDocument::parse_bundle(rsx! {
			<div>
				<style {replace_hash()} />
				<style {replace_hash()} />
				<style {replace_hash()} />
			</div>
		})
		.xpect_snapshot();
	}
	#[test]
	fn assigns_id_to_all() {
		HtmlDocument::parse_bundle(rsx! {
			<div>
				<style {replace_hash()} />
				<span />
			</div>
		})
		.xpect_snapshot();
	}
	#[test]
	fn ignores_templates() {
		HtmlDocument::parse_bundle(rsx! {
			<style {replace_hash()} />
			<MyTemplate />
		})
		.xpect_snapshot();
	}
	// #[test]
	// fn visits_templates() {
	// 	HtmlDocument::parse_bundle(rsx! {
	// 		<style {replace_hash()}/>
	// 		<MyTemplate/>
	// 		<MyTemplate/>


	#[test]
	fn applies_to_slots() {
		HtmlDocument::parse_bundle(rsx! {
			<style {replace_hash()} />
			<MyTemplate>
				<span />
			</MyTemplate>
		})
		.xpect_snapshot();
	}
	#[test]
	fn expressions() {
		let foo = rsx! { <div /> };

		HtmlDocument::parse_bundle(rsx! {
			<style {replace_hash()} />
			{foo}
		})
		.xpect_snapshot();
	}

	#[test]
	fn cascades() {
		HtmlDocument::parse_bundle(rsx! {
			<style {replace_hash()} />
			<MyTemplate style:cascade />
		})
		.xpect_snapshot();
	}

	#[test]
	fn nested_template() {
		#[template]
		fn StyledTemplate(repeat: bool) -> impl Bundle {
			rsx! {
				<style {replace_hash()}/>
				<div/>
				{repeat.xmap_true(||rsx!{<StyledTemplate repeat=false/>})}
			}
		}

		HtmlDocument::parse_bundle(rsx! { <StyledTemplate repeat=true /> })
			.xpect_snapshot();
	}

	#[test]
	#[cfg(feature = "css")]
	fn style_template() {
		use crate::prelude::*;

		#[template]
		fn Style() -> impl Bundle {
			rsx_combinator! {r"
<div>pow</div>
<style>
	body { color: red; }
</style>
			"}
		}

		HtmlDocument::parse_bundle(rsx! { <Style /> }).xpect_snapshot();
	}
}

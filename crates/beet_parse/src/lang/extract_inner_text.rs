use crate::prelude::*;
use beet_dom::prelude::*;
use bevy::prelude::*;


/// Any file with a relative `src` attribute will have its contents extracted
/// and replaced with a [`FileInnerText`] component containing the file contents.
///
/// ## Relative Paths
/// Relative paths are defined by [`path_ext::is_relative_url`],
/// any path not starting with `/`, `http://`, `https://` etc is considered relative.
///
/// In The [`Build`] phase each [`FileInnerText`] is manually loaded via `fs` and
/// replaced with an [`InnerText`] component so this will not be visited.
pub fn extract_inner_text_file(
	mut commands: Commands,
	query: Populated<Entity, Added<ElementNode>>,
	attributes: FindAttribute,
) {
	for entity in query.iter() {
		if let Some((attr_entity, Some(value))) = attributes.find(entity, "src")
			&& path_ext::is_relative_url(&value.0)
		{
			// TODO allow absolute paths?
			commands
				.entity(entity)
				.insert(FileInnerText(value.0.clone()));
			commands.entity(attr_entity).despawn();
		}
	}
}

/// Extract inner text from a non-slot [`ElementNode`] with a single child [`TextNode`]
pub fn extract_inner_text_element(
	mut commands: Commands,
	lit_nodes: Query<&TextNode>,
	query: Populated<
		(Entity, &NodeTag, &Children),
		(With<ElementNode>, Added<NodeTag>),
	>,
) {
	for (entity, node_tag, children) in query.iter() {
		if **node_tag == "slot" {
			// skip slot elements
			continue;
		}

		if children.len() != 1 {
			// only exactly one child is allowed
			continue;
		}

		let Some(&child) = children.first() else {
			// no children, nothing to extract
			continue;
		};

		// replace child text node with InnerText
		if let Ok(text) = lit_nodes.get(child) {
			commands.entity(entity).insert(InnerText(text.to_string()));
			commands.entity(child).despawn();
		}
	}
}


/// For elements with an `innerText` directive, extract the inner text
/// and insert it as an [`InnerText`] component.
/// This is used for elements like `<code inner:text="..."/>`
pub fn extract_inner_text_directive(
	mut commands: Commands,
	attributes: Query<(Entity, &AttributeKey, &NodeExpr)>,
	query: Populated<(Entity, &Attributes), Added<NodeTag>>,
) {
	for (entity, attrs) in query.iter() {
		for (attr_entity, key, value) in
			attrs.iter().filter_map(|attr| attributes.get(attr).ok())
		{
			if key.as_str() == "inner:text" {
				commands.entity(attr_entity).despawn();
				let value = value.inner_parsed();
				commands.entity(entity).insert(NodeExpr::new_block(
					syn::parse_quote!({
						InnerText::new(#value)
					}),
				));
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_dom::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn directive() {
		let mut world = World::new();
		let entity = world
			.spawn((
				NodeTag::new("code"),
				related!(
					Attributes[(
						AttributeKey::new("inner:text"),
						NodeExpr::new_block(syn::parse_quote! {{some_val}})
					)]
				),
			))
			.id();
		world
			.run_system_cached(super::extract_inner_text_directive)
			.unwrap();
		let entity = world.entity(entity);
		entity.contains::<Attributes>().xpect_false();
		entity
			.get::<NodeExpr>()
			.unwrap()
			.self_token_stream()
			.xpect_snapshot();
	}
	#[test]
	fn extracts_src() {
		let mut world = World::new();
		let entity = world
			.spawn((
				ElementNode::self_closing(),
				related!(
					Attributes[(
						AttributeKey::new("src"),
						TextNode::new("./style.css".to_string())
					)]
				),
			))
			.id();
		world
			.run_system_cached(super::extract_inner_text_file)
			.unwrap();
		let entity = world.entity(entity);
		entity
			.get::<FileInnerText>()
			.unwrap()
			.xpect_eq(FileInnerText("./style.css".to_string()));
		entity.contains::<Attributes>().xpect_false();
	}

	#[test]
	fn text_child() {
		let mut world = World::new();
		let entity = world
			.spawn((ElementNode::open(), NodeTag::new("style"), children![
				TextNode::new("div { color: red; }")
			]))
			.id();
		world
			.run_system_cached(super::extract_inner_text_element)
			.unwrap();
		let entity = world.entity(entity);
		entity
			.get::<InnerText>()
			.unwrap()
			.xpect_eq(InnerText::new("div { color: red; }"));
		entity.contains::<Children>().xpect_false();
	}
}

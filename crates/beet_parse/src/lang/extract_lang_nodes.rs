use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use std::hash::Hash;
use std::hash::Hasher;


/// For elements with an `innerText` directive, extract the inner text
/// and insert it as an [`InnerText`] component.
/// This is used for elements like `<code inner:text="..."/>`
pub fn extract_inner_text(
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
// for (entity


/// For elements with a `script`, `style` or `code` tag, and without an
/// `is:inline` attribute, parse as a lang node:
/// - handle the `src` attribute
/// - handle the inner text
/// - insert a [`LangSnippetHash`]
pub fn extract_lang_nodes(
	mut commands: Commands,
	lit_nodes: Query<&TextNode>,
	query: Populated<(Entity, &NodeTag, Option<&Children>), Added<NodeTag>>,
	attributes: FindAttribute,
) {
	for (entity, tag, children) in query.iter() {
		// entirely skip is:inline
		if let Some((attr_ent, _)) = attributes.find(entity, "is:inline") {
			// its done its job, remove it
			commands.entity(attr_ent).despawn();
			continue;
		}

		// Insert the element type
		match tag.as_str() {
			"script" => {
				commands.entity(entity).insert(ScriptElement);
			}
			"style" => {
				commands.entity(entity).insert(StyleElement);
			}
			_ => {
				// skip non-lang nodes
				continue;
			}
		}

		// Collect child TextNode
		let text_child =
			children.iter().flat_map(|c| c.iter()).find_map(|child| {
				match lit_nodes.get(child) {
					Ok(text) => Some((child, text)),
					Err(_) => None,
				}
			});


		// Apply the hash
		let mut hasher = rapidhash::RapidHasher::default();
		tag.hash(&mut hasher);
		for (_, key, value) in attributes.all(entity) {
			key.hash(&mut hasher);
			if let Some(value) = value {
				value.hash(&mut hasher);
			}
		}
		if let Some((_, text)) = text_child {
			// white space sensitive hash of text content, important for <code>
			text.hash(&mut hasher);
		}
		commands
			.entity(entity)
			// all lang nodes must have an open element node, even if they
			// are empty. closed tags like <style src="foo.css"/> are allowed in rsx
			// when specifying a src but will *destroy* a html document
			.insert(ElementNode::open())
			.insert(LangSnippetHash::new(hasher.finish()));


		// replace child text node with InnerText
		if let Some((child, text)) = text_child {
			commands.entity(entity).insert(InnerText(text.to_string()));
			commands.entity(child).despawn();
		}
		// Collect FileInnerText
		else if let Some((attr_entity, Some(value))) =
			attributes.find(entity, "src")
		{
			// TODO allow absolute paths?
			commands
				.entity(entity)
				.insert(FileInnerText(value.0.clone()));
			commands.entity(attr_entity).despawn();
		}

		// ignore nodes without inner text or src, they may have an inner:text
	}
}


#[cfg(test)]
mod test {
	use beet_core::prelude::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	use crate::prelude::NodeExpr;


	#[test]
	fn hashes() {
		let mut world = World::new();
		let entity1 = world
			.spawn((
				NodeTag::new("style"),
				FileSpanOf::<ElementNode>::new(FileSpan::default()),
				children![TextNode::new("div { color: red; }")],
			))
			.id();
		let entity2 = world
			.spawn((
				NodeTag::new("style"),
				FileSpanOf::<ElementNode>::new(FileSpan::default()),
				children![TextNode::new("div { color: blue; }")],
			))
			.id();
		let entity3 = world
			.spawn((
				NodeTag::new("style"),
				FileSpanOf::<ElementNode>::new(FileSpan::default()),
				children![TextNode::new("div { color: blue; }")],
			))
			.id();
		world.run_system_once(super::extract_lang_nodes).unwrap();
		let hash1 = world.entity(entity1).get::<LangSnippetHash>().unwrap();
		let hash2 = world.entity(entity2).get::<LangSnippetHash>().unwrap();
		let hash3 = world.entity(entity3).get::<LangSnippetHash>().unwrap();
		expect(hash1).not().to_be(hash2);
		expect(hash2).to_be(hash3);
	}
	#[test]
	fn extracts_inner_text() {
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
		world.run_system_once(super::extract_inner_text).unwrap();
		let entity = world.entity(entity);
		entity.contains::<Attributes>().xpect().to_be(false);
		entity
			.get::<NodeExpr>()
			.unwrap()
			.self_token_stream()
			.xpect()
			.to_be_snapshot();
	}



	#[test]
	fn extracts_inline() {
		let mut world = World::new();
		let entity = world
			.spawn((
				NodeTag::new("style"),
				FileSpanOf::<ElementNode>::new(FileSpan::default()),
				children![TextNode::new("div { color: red; }")],
			))
			.id();
		world.run_system_once(super::extract_lang_nodes).unwrap();
		let entity = world.entity(entity);
		entity
			.get::<InnerText>()
			.unwrap()
			.xpect()
			.to_be(&InnerText::new("div { color: red; }"));
		entity.contains::<Children>().xpect().to_be(false);
	}
	#[test]
	fn extracts_src() {
		let mut world = World::new();
		let entity = world
			.spawn((
				NodeTag::new("style"),
				FileSpanOf::<ElementNode>::new(FileSpan::new(
					file!(),
					default(),
					default(),
				)),
				related!(
					Attributes[(
						AttributeKey::new("src"),
						TextNode::new("./style.css".to_string())
					)]
				),
			))
			.id();
		world.run_system_once(super::extract_lang_nodes).unwrap();
		let entity = world.entity(entity);
		entity
			.get::<FileInnerText>()
			.unwrap()
			.xpect()
			.to_be(&FileInnerText("./style.css".to_string()));
		entity.contains::<Attributes>().xpect().to_be(false);
	}
}

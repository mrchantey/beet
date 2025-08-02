use beet_core::prelude::*;
use bevy::prelude::*;
use std::hash::Hash;
use std::hash::Hasher;


/// For elements with a `script`, `style` or `code` tag, and without an
/// `is:inline` attribute, parse as a lang node:
/// - insert a [`LangSnippetHash`]
/// - insert a [`ScriptElement`]
/// - insert a [`StyleElement`]
pub fn extract_lang_nodes(
	mut commands: Commands,
	query: Populated<(Entity, &NodeTag, Option<&InnerText>), Added<NodeTag>>,
	attributes: FindAttribute,
) {
	for (entity, tag, inner_text) in query.iter() {
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

		// Apply the hash
		let mut hasher = rapidhash::RapidHasher::default();
		tag.hash(&mut hasher);
		for (_, key, value) in attributes.all(entity) {
			key.hash(&mut hasher);
			if let Some(value) = value {
				value.hash(&mut hasher);
			}
		}
		// white space sensitive hash of text content, important for <code>
		inner_text.hash(&mut hasher);
		commands
			.entity(entity)
			.insert(LangSnippetHash::new(hasher.finish()));
	}
}


#[cfg(test)]
mod test {
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn hashes() {
		let mut world = World::new();
		let entity1 = world
			.spawn((
				NodeTag::new("style"),
				FileSpanOf::<ElementNode>::new(FileSpan::default()),
				InnerText::new("div { color: red; }"),
			))
			.id();
		let entity2 = world
			.spawn((
				NodeTag::new("style"),
				FileSpanOf::<ElementNode>::new(FileSpan::default()),
				InnerText::new("div { color: blue; }"),
			))
			.id();
		let entity3 = world
			.spawn((
				NodeTag::new("style"),
				FileSpanOf::<ElementNode>::new(FileSpan::default()),
				InnerText::new("div { color: blue; }"),
			))
			.id();
		world.run_system_cached(super::extract_lang_nodes).unwrap();
		let hash1 = world.entity(entity1).get::<LangSnippetHash>().unwrap();
		let hash2 = world.entity(entity2).get::<LangSnippetHash>().unwrap();
		let hash3 = world.entity(entity3).get::<LangSnippetHash>().unwrap();
		expect(hash1).not().to_be(hash2);
		expect(hash2).to_be(hash3);
	}
}

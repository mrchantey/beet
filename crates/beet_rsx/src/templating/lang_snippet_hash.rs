use super::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use std::hash::Hash;
use std::hash::Hasher;



/// Insert a [`LangSnippetHash`] for script and style nodes.
pub fn apply_lang_snippet_hashes(
	mut commands: Commands,
	query: Populated<
		(
			Entity,
			&NodeTag,
			// all components that would effect this nodes uniqueness
			Option<&InnerText>,
			Option<&StyleScope>,
			Option<&HtmlHoistDirective>,
		),
		Or<(Added<ScriptElement>, Added<StyleElement>)>,
	>,
	attributes: FindAttribute,
) {
	for (entity, tag, inner_text, scope, hoist) in query.iter() {
		// Apply the hash
		let mut hasher = FixedHasher::default().build_hasher();
		tag.hash(&mut hasher);
		for (_, key, value) in attributes.all(entity) {
			key.hash(&mut hasher);
			value.hash(&mut hasher);
		}
		inner_text.hash(&mut hasher);
		scope.hash(&mut hasher);
		hoist.hash(&mut hasher);
		commands
			.entity(entity)
			.insert(LangSnippetHash::new(hasher.finish()));
	}
}


/// All identical <script> and <style> elements can be deduplicated,
/// with the remaining element usually hoisted to the head unless
/// otherwise specified
// TODO this should probably actually merge the elements, cleaner in prod
pub fn deduplicate_lang_nodes(
	mut commands: Commands,
	roots: Populated<Entity, Added<HtmlDocument>>,
	children: Query<&Children>,
	hashes: Query<(Entity, &LangSnippetHash)>,
) {
	for root in roots.iter() {
		let mut visited = HashSet::new();
		for (entity, hash) in children
			.iter_descendants(root)
			.filter_map(|child| hashes.get(child).ok())
		{
			if visited.contains(hash) {
				commands.entity(entity).despawn();
			} else {
				visited.insert(hash.clone());
				// add the hash as an attribute for debugging
				#[cfg(all(debug_assertions, not(test)))]
				commands.spawn((
					AttributeOf::new(entity),
					AttributeKey::new("data-lang-hash"),
					TextNode::new(hash.to_string()),
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

	#[test]
	fn hashes() {
		let mut world = World::new();
		let entity1 = world
			.spawn(rsx! {<style>div { color: red; }</div>})
			.get::<Children>()
			.unwrap()[0];
		let entity2 = world
			.spawn(rsx! {<style>div { color: blue; }</div>})
			.get::<Children>()
			.unwrap()[0];
		let entity3 = world
			.spawn(rsx! {<style>div { color: blue; }</div>})
			.get::<Children>()
			.unwrap()[0];
		world
			.run_system_cached(super::apply_lang_snippet_hashes)
			.unwrap();
		let hash1 = world.entity(entity1).get::<LangSnippetHash>().unwrap();
		let hash2 = world.entity(entity2).get::<LangSnippetHash>().unwrap();
		let hash3 = world.entity(entity3).get::<LangSnippetHash>().unwrap();
		hash1.xpect_not_eq(*hash2);
		hash2.xpect_eq(*hash3);
	}
}

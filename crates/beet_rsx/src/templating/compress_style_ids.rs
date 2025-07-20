use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// converts all style id hashes to a numeric index
pub fn compress_style_ids(
	mut commands: Commands,
	constants: Res<HtmlConstants>,
	roots: Populated<Entity, Added<HtmlDocument>>,
	children: Query<&Children>,
	hash_attrs: Query<(Entity, &LangSnippetHash)>,
	mut lang_elements: Query<(&LangSnippetHash, &mut InnerText)>,
	elements: Query<(Entity, Option<&Attributes>)>,
) -> Result {
	for root in roots.iter() {
		// initialize counter
		let mut index_incr = 0;
		let mut index_map = HashMap::<LangSnippetHash, u64>::default();
		let mut get_next_index = |hash: &LangSnippetHash| {
			let index = index_map.entry(hash.clone()).or_insert_with(|| {
				let idx = index_incr;
				index_incr += 1;
				idx
			});
			*index
		};

		for (entity, attributes) in children
			.iter_descendants(root)
			.filter_map(|ent| elements.get(ent).ok())
		{
			// convert indices found in the css
			if let Ok((hash, mut text)) = lang_elements.get_mut(entity) {
				let index = get_next_index(hash);
				let original_id = constants.style_id_attribute(**hash);
				let new_id = constants.style_id_attribute(index);
				text.0 = text.0.replace(&original_id, &new_id);
			};

			// convert indices found in the attributes
			if let Some(attrs) = attributes {
				for (attr_entity, attr_hash) in
					attrs.iter().filter_map(|a| hash_attrs.get(a).ok())
				{
					let index = get_next_index(attr_hash);

					commands
						.entity(attr_entity)
						.remove::<LangSnippetHash>()
						.insert(AttributeKey::new(
							constants.style_id_attribute(index),
						));
				}
			}
		}
	}

	Ok(())
}

#[cfg(test)]
#[cfg(feature = "css")]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		HtmlDocument::parse_bundle(rsx! {
			<div>
			<style>
				foo{color:red;}
			</style>
		})
		.xpect()
		.to_be_snapshot();
	}
}

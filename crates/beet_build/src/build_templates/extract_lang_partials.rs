use crate::prelude::*;
use beet_common::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use rapidhash::RapidHasher;
use serde::Deserialize;
use serde::Serialize;
use std::hash::Hash;
use std::hash::Hasher;
use sweet::prelude::ReadFile;

/// The loaded and deduplicated [`LangContent`].
#[derive(
	Debug,
	Clone,
	PartialEq,
	Hash,
	Deref,
	Serialize,
	Deserialize,
	Component,
	Reflect,
)]
#[reflect(Component)]
// #[component(immutable)]
pub struct LangPartial(pub String);

/// Deduplicate `script` and `style` tags by replacing the
/// [`LangContent`] directive with a [`NodePortal`] to a shared [`LangPartial`].
pub fn extract_lang_partials(
	mut commands: Commands,
	query: Populated<
		(Entity, &NodeTag, &LangContent, Option<&StyleScope>),
		Added<LangContent>,
	>,
) -> Result<()> {
	let mut groups = HashMap::<u64, Vec<Entity>>::new();

	for (entity, tag, lang_content, scope) in query.iter() {
		let mut hasher = RapidHasher::default();
		tag.hash(&mut hasher);
		lang_content.hash_no_whitespace(&mut hasher);
		scope.map(|s| s.hash(&mut hasher));
		let hash = hasher.finish();
		groups.entry(hash).or_default().push(entity);
	}
	for entities in groups.into_values() {
		let (_, tag, content, scope) = query.get(entities[0]).expect("checked");

		let content = match content {
			LangContent::InnerText(text) => text.to_string(),
			LangContent::File(path) => {
				let file = ReadFile::to_string(path.into_abs()?)?;
				file.to_string()
			}
		};

		let mut target = commands.spawn((tag.clone(), LangPartial(content)));


		if let Some(scope) = scope {
			target.insert(scope.clone());
		}
		let target = target.id();
		for entity in entities.iter() {
			commands
				.entity(*entity)
				.remove::<LangContent>()
				.insert(NodePortal::new(target));
		}
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use bevy::prelude::*;
	// use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins((NodeTokensPlugin, BuildTemplatesPlugin));

		let entity = app.world_mut().spawn(rsx! {<style>div{}</style>}).id();
		app.update();
		let portal = app.world().get::<NodePortal>(entity).unwrap();

		let _partial = app.world().get::<LangPartial>(**portal).unwrap();
	}
}

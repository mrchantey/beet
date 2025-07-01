use beet_common::prelude::*;
use beet_utils::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use rapidhash::RapidHasher;
use std::hash::Hash;
use std::hash::Hasher;

/// Deduplicate `script` and `style` tags by replacing the
/// [`LangContent`] directive with a [`NodePortal`] to a shared [`LangPartial`].
pub fn extract_lang_partials(
	mut commands: Commands,
	query: Populated<
		(
			Entity,
			&NodeTag,
			&LangContent,
			Option<&StyleScope>,
			Option<&HtmlHoistDirective>,
		),
		Added<LangContent>,
	>,
) -> Result<()> {
	let mut groups = HashMap::<u64, Vec<Entity>>::new();

	for (entity, tag, lang_content, scope, hoist) in query.iter() {
		let mut hasher = RapidHasher::default();
		tag.hash(&mut hasher);
		lang_content.hash_no_whitespace(&mut hasher);
		scope.map(|s| s.hash(&mut hasher));
		hoist.map(|h| h.hash(&mut hasher));
		let hash = hasher.finish();
		groups.entry(hash).or_default().push(entity);
	}
	for entities in groups.into_values() {
		let (_, tag, content, scope, hoist) =
			query.get(entities[0]).expect("checked");

		let content = match content {
			LangContent::InnerText(text) => text.to_string(),
			LangContent::File(path) => {
				let file = ReadFile::to_string(path.into_abs())?;
				file.to_string()
			}
		};

		let mut target = commands.spawn((tag.clone(), LangPartial(content)));
		scope.map(|scope| {
			target.insert(scope.clone());
		});
		hoist.map(|hoist| {
			target.insert(hoist.clone());
		});

		let target = target.id();
		for entity in entities.iter() {
			// these entities are now just portals to the shared content
			commands
				.entity(*entity)
				.remove::<LangContent>()
				.remove::<NodeTag>()
				.remove::<ElementNode>()
				.remove::<Children>()
				.remove::<Attributes>()
				.insert(PortalTo::<LangPartial>::default())
				.insert(NodePortal::new(target));
		}
	}
	Ok(())
}

/// For each style [`LangPartial`] with a [`StyleScope::Local`],
/// assign a unique [`StyleId`] to the entity and each
/// [`NodePortalTarget`].
pub fn apply_style_ids(
	mut commands: Commands,
	query: Populated<(
		Entity,
		&NodeTag,
		&LangPartial,
		&NodePortalTarget,
		Option<&StyleScope>,
	)>,
) {
	for (id, (entity, tag, _partial, portal_sources, scope)) in
		query.iter().enumerate()
	{
		// only local style tags
		if tag.as_str() != "style"
			|| scope.map(|s| s == &StyleScope::Global).unwrap_or(false)
		{
			continue;
		}
		let styleid = StyleId::new(id as u64);
		commands.entity(entity).insert(styleid);
		for source in portal_sources.iter() {
			commands.entity(source).insert(styleid);
		}
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins((ParseRsxTokensPlugin, StaticScenePlugin));

		let entity = app
			.world_mut()
			.spawn(rsx! {<style>div{color:blue;}</style>})
			.get::<Children>()
			.unwrap()[0];
		app.update();
		let portal = app.world().get::<NodePortal>(entity).unwrap();

		#[cfg(feature = "css")]
		let expected = "div[data-beet-style-id-0] {\n  color: #00f;\n}\n";
		#[cfg(not(feature = "css"))]
		let expected = "div{color:blue;}";

		app.world()
			.get::<LangPartial>(**portal)
			.unwrap()
			.0
			.clone()
			.xpect()
			.to_be(expected);
	}
}

use crate::prelude::*;
use beet_common::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::hash::Hash;


/// Added to each [`LangPartial`] and each [`NodePortalTarget`] source
#[derive(
	Debug,
	Copy,
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
#[component(immutable)]
pub struct StyleId(u64);


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
		let styleid = StyleId(id as u64);
		commands.entity(entity).insert(styleid);
		for source in portal_sources.iter() {
			commands.entity(source).insert(styleid);
		}
	}
}

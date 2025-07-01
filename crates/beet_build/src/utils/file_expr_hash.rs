use super::HashNonTemplateRust;
use crate::prelude::*;
use beet_bevy::prelude::HierarchyQueryExtExt;
use beet_common::prelude::*;
use beet_parse::prelude::*;
use beet_router::as_beet::TemplateRoot;
use bevy::prelude::*;
use quote::ToTokens;
use rapidhash::RapidHasher;
use std::hash::Hash;
use std::hash::Hasher;

/// A hash of all non-literal expressions in a file containing rust code,
/// including `.rs`, `.mdx` and `.rsx` files.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Deref)]
pub struct FileExprHash(u64);

impl FileExprHash {
	pub fn new(hash: u64) -> Self { Self(hash) }

	pub fn hash(&self) -> u64 { self.0 }
}



/// Update the [`FileExprHash`] component for all template files if it changed.
/// Use change detection to trigger extra work based on the hash change.
pub fn update_file_expr_hash(
	_: TempNonSendMarker,
	macros: Res<TemplateMacros>,
	mut query: Populated<
		(Entity, &SourceFile, &mut FileExprHash),
		Changed<SourceFile>,
	>,
	template_roots: Query<&TemplateRoot>,
	template_tags: Query<&NodeTag, With<TemplateNode>>,
	children: Query<&Children>,
	macro_idxs: Query<&MacroIdx>,
	attributes: Query<&Attributes>,
	node_exprs: Query<&NodeExpr, Without<AttributeOf>>,
	// dont hash literal attribute values
	attr_exprs: Query<&NodeExpr, (With<AttributeOf>, Without<AttributeLit>)>,
) -> Result {
	for (entity, source_file, mut hash) in query.iter_mut() {
		let mut hasher = RapidHasher::default_const();
		HashNonTemplateRust {
			macros: &macros,
			hasher: &mut hasher,
		}
		.hash(source_file)?;
		for node in children
			.iter_descendants(entity)
			.flat_map(|child| template_roots.iter_descendants(child))
			.flat_map(|en| children.iter_descendants_inclusive(en))
		{
			// hash macro idxs
			if let Ok(idx) = macro_idxs.get(node) {
				idx.hash(&mut hasher);
			}

			// has template tags
			if let Ok(tag) = template_tags.get(node) {
				tag.to_string().hash(&mut hasher);
			}

			// hash block nodes
			if let Ok(expr) = node_exprs.get(node) {
				expr.to_token_stream().to_string().hash(&mut hasher);
			}
			// hash attribute expressions
			for expr in attributes
				.iter_descendants(node)
				.filter_map(|entity| attr_exprs.get(entity).ok())
			{
				expr.to_token_stream().to_string().hash(&mut hasher);
			}
		}
		let new_hash = hasher.finish();
		hash.set_if_neq(FileExprHash::new(new_hash));
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use beet_bevy::prelude::WorldMutExt;
	use beet_utils::prelude::*;
	use bevy::prelude::*;
	use send_wrapper::SendWrapper;
	use sweet::prelude::*;

	fn hash(bundle: impl Bundle) -> u64 { hash_inner(bundle, true) }

	fn hash_inner(bundle: impl Bundle, remove_macro_idxs: bool) -> u64 {
		let mut app = App::new();
		app.init_resource::<TemplateMacros>()
			.add_systems(Update, update_file_expr_hash);
		let entity = app
			.world_mut()
			.spawn((
				SourceFile::new(WsPathBuf::new(file!()).into_abs()),
				children![related! {TemplateRoot[bundle]}],
			))
			.id();
		// reset macro idxs for testing
		if remove_macro_idxs {
			for entity in app
				.world_mut()
				.query_filtered_once::<Entity, With<MacroIdx>>()
			{
				app.world_mut().entity_mut(entity).remove::<MacroIdx>();
			}
		}
		app.update();
		app.world().get::<FileExprHash>(entity).unwrap().0
	}

	#[test]
	#[rustfmt::skip]
	fn tag_names() {
		expect(hash(rsx_tokens! {<div/>}))
		.to_be(hash(rsx_tokens! {<span/>}));

		
		expect(hash(rsx_tokens! {<Foo/>}))
    .not()		
		.to_be(hash(rsx_tokens! {<Bar/>}));
	}
	#[test]
	fn attributes() {
		expect(hash(rsx_tokens! {<div foo/>}))
			.to_be(hash(rsx_tokens! {<div bar/>}));
	}
	#[test]
	fn node_blocks() {
		expect(hash(rsx_tokens! {<div>{1}</div>}))
			.to_be(hash(rsx_tokens! {<div>{1}</div>}));

		expect(hash(rsx_tokens! {<div>{1}</div>}))
			.not()
			.to_be(hash(rsx_tokens! {<div>{2}</div>}));
	}
	#[test]
	fn macro_idxs() {
		// different LineCol means different hash
		expect(hash_inner(rsx_tokens! {<div>{1}</div>}, false))
			.not()
			.to_be(hash_inner(rsx_tokens! {<div>{1}</div>}, false));
	}

	// TODO combinator attributes
}

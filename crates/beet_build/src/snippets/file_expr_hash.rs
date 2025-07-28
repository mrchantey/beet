use super::HashNonSnippetRust;
use crate::prelude::*;
use beet_core::prelude::HierarchyQueryExtExt;
use beet_core::prelude::*;
use beet_parse::prelude::*;
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



/// Idents used for template macros.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Resource)]
pub struct TemplateMacros {
	pub rstml: String,
}
impl Default for TemplateMacros {
	fn default() -> Self {
		Self {
			rstml: "rsx".to_string(),
		}
	}
}


/// Update the [`FileExprHash`] component for all template files if it changed.
/// Use change detection to trigger extra work based on the hash change.
pub fn update_file_expr_hash(
	// even though our tokens are Unspan, we're interactig with ParseRsxTokens
	// which also handles !Send tokens, so we must ensure main thread.
	_: TempNonSendMarker,
	macros: Res<TemplateMacros>,
	mut query: Populated<
		(Entity, &SourceFile, &mut FileExprHash),
		Changed<SourceFile>,
	>,
	template_roots: Query<&TemplateRoot>,
	template_tags: Query<&NodeTag, With<TemplateNode>>,
	children: Query<&Children>,
	rsx_snippets: Query<&RsxSnippets>,
	snippet_roots: Query<&SnippetRoot>,
	node_exprs: Query<&NodeExpr, Without<AttributeOf>>,
	attributes: Query<&Attributes>,
	// dont hash literal attribute values, they can be updated via snippets
	attr_exprs: Query<&NodeExpr, (With<AttributeOf>, Without<TextNode>)>,
	// hash all template attributes, they are currently used to build functions
	// should change when bevy has native templates
	template_attrs: Query<(
		Option<&AttributeKey>,
		Option<&TextNode>,
		Option<&NodeExpr>,
	)>,
) -> Result {
	for (entity, source_file, mut hash) in query.iter_mut() {
		let mut hasher = RapidHasher::default_const();
		HashNonSnippetRust {
			macros: &macros,
			hasher: &mut hasher,
		}
		.hash(source_file)?;
		for node in rsx_snippets
			.iter_descendants(entity)
			.flat_map(|child| template_roots.iter_descendants_inclusive(child))
			.flat_map(|en| children.iter_descendants_inclusive(en))
		{
			// hash macro idxs
			if let Ok(idx) = snippet_roots.get(node) {
				idx.hash(&mut hasher);
			}

			// has template tags
			if let Ok(tag) = template_tags.get(node) {
				tag.to_string().hash(&mut hasher);
				// hash all template attributes
				for (key, lit, expr) in attributes
					.iter_descendants(node)
					.filter_map(|entity| template_attrs.get(entity).ok())
				{
					if let Some(key) = key {
						key.to_string().hash(&mut hasher);
					}
					if let Some(lit) = lit {
						lit.to_string().hash(&mut hasher);
					}
					if let Some(expr) = expr {
						expr.to_token_stream().to_string().hash(&mut hasher);
					}
				}
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

		let status = if hash.0 == new_hash {
			"SAME"
		} else {
			"CHANGED"
		};
		debug!("FileExprHash {status} {}", source_file.path());
		hash.set_if_neq(FileExprHash::new(new_hash));
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use crate::prelude::*;
	use beet_core::prelude::WorldMutExt;
	use beet_utils::prelude::*;
	use bevy::prelude::*;
	use send_wrapper::SendWrapper;
	use sweet::prelude::*;

	fn hash(bundle: impl Bundle) -> u64 { hash_inner(bundle, true) }

	fn hash_inner(bundle: impl Bundle, remove_snippet_roots: bool) -> u64 {
		let mut app = App::new();
		app.init_resource::<TemplateMacros>()
			.add_systems(Update, update_file_expr_hash);
		let entity = app
			.world_mut()
			.spawn((
				SourceFile::new(WsPathBuf::new(file!()).into_abs()),
				related! {RsxSnippets[related! {TemplateRoot[bundle]}]},
			))
			.id();
		// reset macro idxs for testing
		if remove_snippet_roots {
			for entity in app
				.world_mut()
				.query_filtered_once::<Entity, With<SnippetRoot>>()
			{
				app.world_mut().entity_mut(entity).remove::<SnippetRoot>();
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
		//same
		expect(hash(rsx_tokens! {<div>{1}</div>}))
			.to_be(hash(rsx_tokens! {<div>{1}</div>}));
		//dif inner
		expect(hash(rsx_tokens! {<div>{1}</div>}))
			.not()
			.to_be(hash(rsx_tokens! {<div>{2}</div>}));
		// diff num
		expect(hash(rsx_tokens! {<div>foo </div>}))
			.not()
			.to_be(hash(rsx_tokens! {<div>bar {2}</div>}));
	}
	#[test]
	fn combinator() {
		//same
		expect(hash(rsx_combinator_tokens! {"<div>{1}</div>"}))
			.to_be(hash(rsx_combinator_tokens! {"<div>{1}</div>"}));
		//dif inner
		expect(hash(rsx_combinator_tokens! {"<div>{1}</div>"}))
			.not()
			.to_be(hash(rsx_combinator_tokens! {"<div>{2}</div>"}));
		// diff num
		expect(hash(rsx_combinator_tokens! {"<div></div>"}))
			.not()
			.to_be(hash(rsx_combinator_tokens! {"<div>{2}</div>"}));
		// diff attribute
		expect(hash(rsx_combinator_tokens! {"<div foo={let a = 2;a}/>"}))
			.not()
			.to_be(hash(rsx_combinator_tokens! {"<div foo={let a = 3;a}/>"}));
	}
	#[test]
	fn templates() {
		// same
		expect(hash(rsx_tokens! {<Foo>{1}</Foo>}))
			.to_be(hash(rsx_tokens! {<Foo>{1}</Foo>}));

		// diff
		expect(hash(rsx_tokens! {<Foo>{1}</Foo>}))
			.not()
			.to_be(hash(rsx_tokens! {<Foo>{2}</Foo>}));
		expect(hash(rsx_tokens! {<Foo bar=1/>}))
			.not()
			.to_be(hash(rsx_tokens! {<Foo bar=2/>}));

		// diff nested
		expect(hash(
			rsx_tokens! {<Foo><Bar><Bazz>bar{1}</Bazz></Bar></Foo>},
		))
		.not()
		.to_be(hash(rsx_tokens! {<Foo><Bar><Bazz>bar</Bazz></Bar></Foo>}));
	}
	#[test]
	fn snippet_roots() {
		// different LineCol means different hash
		expect(hash_inner(rsx_tokens! {<div>{1}</div>}, false))
			.not()
			.to_be(hash_inner(rsx_tokens! {<div>{1}</div>}, false));
	}

	#[test]
	fn doesnt_change() {
		let mut app = App::new();
		app.add_plugins(BuildPlugin::without_fs());

		let index_path = WsPathBuf::new(
			"crates/beet_router/src/test_site/pages/docs/index.rs",
		);
		let mut query = app
			.world_mut()
			.query_filtered::<(), Changed<FileExprHash>>();
		app.world_mut()
			.spawn(SourceFile::new(index_path.into_abs()));

		expect(query.iter(app.world()).count()).to_be(1);
		app.update();
		expect(query.iter(app.world()).count()).to_be(0);
	}
}

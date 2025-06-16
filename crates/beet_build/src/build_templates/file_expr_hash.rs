use super::HashNonTemplateRust;
use crate::prelude::*;
use beet_bevy::prelude::HierarchyQueryExtExt;
use beet_common::prelude::*;
use beet_parse::exports::SendWrapper;
use beet_template::prelude::*;
use bevy::prelude::*;
use quote::ToTokens;
use rapidhash::RapidHasher;
use std::hash::Hash;
use std::hash::Hasher;
use syn::Expr;

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
	source_files: Query<&TemplateRoots>,
	template_roots: Query<&TemplateRoot>,
	template_tags: Query<&NodeTag, With<TemplateNode>>,
	children: Query<&Children>,
	attributes: Query<&Attributes>,
	block_nodes: Query<&ItemOf<BlockNode, SendWrapper<Expr>>>,
	attr_exprs: Query<&AttributeExpr>,
	attr_key_exprs: Query<&AttributeKeyExpr>,
	attr_val_exprs: Query<&AttributeValueExpr>,
	mut query: Populated<
		(Entity, &TemplateFile, &mut FileExprHash),
		Changed<TemplateFile>,
	>,
) -> Result {
	for (entity, template_file, mut hash) in query.iter_mut() {
		let mut hasher = RapidHasher::default_const();
		HashNonTemplateRust {
			macros: &macros,
			hasher: &mut hasher,
		}
		.hash(template_file)?;

		for template in source_files.iter_descendants(entity) {
			for root in template_roots.iter_descendants(template) {
				for node in children.iter_descendants_inclusive(root) {
					// has template tags
					if let Ok(tag) = template_tags.get(node) {
						tag.to_string().hash(&mut hasher);
					}

					// hash block nodes
					if let Ok(block_node) = block_nodes.get(node) {
						block_node
							.to_token_stream()
							.to_string()
							.hash(&mut hasher);
					}

					for attribute in attributes.iter_descendants(node) {
						// has attribute expressions
						if let Ok(attr_expr) = attr_exprs.get(attribute) {
							attr_expr
								.to_token_stream()
								.to_string()
								.hash(&mut hasher);
						}
						// hash non-literal attribute keys
						if let Ok(attr_key_expr) = attr_key_exprs.get(attribute)
							&& !matches!(attr_key_expr.inner(), Expr::Lit(_))
						{
							attr_key_expr
								.to_token_stream()
								.to_string()
								.hash(&mut hasher);
						}
						// hash non-literal attribute values
						if let Ok(attr_val_expr) = attr_val_exprs.get(attribute)
							&& !matches!(attr_val_expr.inner(), Expr::Lit(_))
						{
							attr_val_expr
								.to_token_stream()
								.to_string()
								.hash(&mut hasher);
						}
					}
				}
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
	use beet_utils::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	fn hash(bundle: impl Bundle) -> u64 {
		let mut app = App::new();
		app.init_resource::<TemplateMacros>()
			.add_systems(Update, update_file_expr_hash);
		let entity = app
			.world_mut()
			.spawn((
				TemplateFile::new(WsPathBuf::new(file!())),
				related! {TemplateRoots[related!{TemplateRoot[bundle]}]},
			))
			.id();
		app.update();
		app.world().get::<FileExprHash>(entity).unwrap().0
	}
	mod syn {
		pub use syn::*;
		pub mod expr {
			pub use syn::Expr;
		}
	}
	mod send_wrapper {
		pub use beet_parse::exports::SendWrapper;
	}
	use send_wrapper::SendWrapper;

	#[test]
	#[rustfmt::skip]
	fn tag_names() {
		// expect(hash(rsx_tokens! {<div/>}))
		// .to_be(hash(rsx_tokens! {<span/>}));

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

	// TODO combinator attributes
}

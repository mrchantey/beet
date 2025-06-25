use super::RstmlCustomNode;
use crate::prelude::*;
use beet_common::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use proc_macro2::Span;
use proc_macro2_diagnostics::Diagnostic;
use proc_macro2_diagnostics::Level;
use rstml::node::KVAttributeValue;
use rstml::node::KeyedAttributeValue;
use rstml::node::Node;
use rstml::node::NodeAttribute;
use rstml::node::NodeBlock;
use rstml::node::NodeElement;
use rstml::node::NodeName;
use send_wrapper::SendWrapper;
use syn::Expr;
use syn::ExprLit;
use syn::spanned::Spanned;



pub fn rstml_to_node_tokens_plugin(app: &mut App) {
	app.add_systems(
		Update,
		rstml_to_node_tokens
			.in_set(ImportNodesStep)
			.after(super::tokens_to_rstml),
	);
}

/// Replace [`RstmlNodes`] with children representing each [`rstml::Node`].
fn rstml_to_node_tokens(
	_: TempNonSendMarker,
	mut commands: Commands,
	rstml_config: Res<RstmlConfig>,
	mut query: Populated<(
		Entity,
		&SourceFile,
		&RstmlRoot,
		&mut TokensDiagnostics,
	)>,
) -> Result {
	for (entity, source_file, rstml_nodes, diagnostics) in query.iter_mut() {
		let root_node = rstml_nodes.clone();

		let mut collected_elements = CollectedElements::default();

		RstmlToWorld {
			file: source_file,
			rstml_config: &rstml_config,
			collected_elements: &mut collected_elements,
			diagnostics,
			commands: &mut commands,
			expr_idx: ExprIdxBuilder::new(),
		}
		.insert_node(entity, root_node.take());
		commands
			.entity(entity)
			.remove::<RstmlRoot>()
			.insert(collected_elements);
	}
	Ok(())
}


struct RstmlToWorld<'w, 's, 'a> {
	file: &'a WsPathBuf,
	rstml_config: &'a RstmlConfig,
	collected_elements: &'a mut CollectedElements,
	diagnostics: Mut<'a, TokensDiagnostics>,
	commands: &'a mut Commands<'w, 's>,
	expr_idx: ExprIdxBuilder,
}

impl<'w, 's, 'a> RstmlToWorld<'w, 's, 'a> {
	/// Returns the entity containing these nodes
	pub fn spawn_nodes(
		&mut self,
		nodes: Vec<Node<RstmlCustomNode>>,
	) -> Vec<Entity> {
		nodes
			.into_iter()
			.map(|node| {
				let entity = self.commands.spawn_empty().id();
				self.insert_node(entity, node);
				entity
			})
			.collect()
	}


	fn insert_node(&mut self, entity: Entity, node: Node<RstmlCustomNode>) {
		let node_span = SendWrapper::new(node.span());
		let file_span = FileSpan::new_from_span(self.file.clone(), &node);
		// let spans = (node_span, file_span);
		match node {
			Node::Doctype(_) => {
				self.commands.entity(entity).insert((
					DoctypeNode,
					ItemOf::<DoctypeNode, _>::new(file_span),
					ItemOf::<DoctypeNode, _>::new(node_span),
				));
			}
			Node::Comment(node) => {
				self.commands.entity(entity).insert((
					CommentNode(node.value.value()),
					ItemOf::<CommentNode, _>::new(file_span),
					ItemOf::<CommentNode, _>::new(node_span),
				));
			}
			Node::Text(node) => {
				self.commands.entity(entity).insert((
					TextNode(node.value.value()),
					ItemOf::<TextNode, _>::new(file_span),
					ItemOf::<TextNode, _>::new(node_span),
				));
			}
			Node::RawText(node) => {
				self.commands.entity(entity).insert((
					TextNode(node.to_string_best()),
					ItemOf::<TextNode, _>::new(file_span),
					ItemOf::<TextNode, _>::new(node_span),
				));
			}
			Node::Fragment(fragment) => {
				let children = self.spawn_nodes(fragment.children);
				self.commands
					.entity(entity)
					.insert((
						FragmentNode,
						ItemOf::<FragmentNode, _>::new(file_span),
						ItemOf::<FragmentNode, _>::new(node_span),
					))
					.add_children(&children);
			}
			Node::Block(NodeBlock::ValidBlock(block)) => {
				let expr = SendWrapper::<Expr>::new(syn::parse_quote!(
					#[allow(unused_braces)]
					#block
				));
				self.commands.entity(entity).insert((
					BlockNode,
					self.expr_idx.next(),
					ItemOf::<BlockNode, _>::new(file_span),
					ItemOf::<BlockNode, _>::new(node_span),
					ItemOf::<BlockNode, _>::new(expr),
				));
			}
			Node::Block(NodeBlock::Invalid(invalid)) => {
				self.diagnostics.push(Diagnostic::spanned(
					invalid.span(),
					Level::Error,
					"Invalid block",
				));
				self.commands.entity(entity).insert((
					BlockNode,
					ItemOf::<BlockNode, _>::new(file_span),
					ItemOf::<BlockNode, _>::new(node_span),
				));
			}
			Node::Element(el) => {
				self.check_self_closing_children(&el);

				let NodeElement {
					open_tag,
					children,
					close_tag,
				} = el;
				let (tag_str, tag_file_span, tag_span) =
					self.parse_node_name(&open_tag.name);

				self.collected_elements
					.push((tag_str.clone(), tag_span.clone()));
				let self_closing = close_tag.is_none();
				if let Some(close_tag) = close_tag.as_ref() {
					let close_tag = self.parse_node_name(&close_tag.name);
					self.collected_elements.push((close_tag.0, close_tag.2));
				}

				// let attributes = AttributeTokensList(attributes);

				let children = self.spawn_nodes(children);

				let mut entity = self.commands.entity(entity);
				entity.insert((
					NodeTag(tag_str.clone()),
					ItemOf::<NodeTag, _>::new(tag_file_span),
					ItemOf::<NodeTag, _>::new(tag_span),
				));
				entity.add_children(&children);

				if tag_str.starts_with(|c: char| c.is_uppercase()) {
					entity.insert((
						TemplateNode,
						// yes we get the ExprIdx after its children, its fine as long
						// as its consistent with other parsers.
						self.expr_idx.next(),
						ItemOf::<TemplateNode, _>::new(file_span),
						ItemOf::<TemplateNode, _>::new(node_span),
					));
				} else {
					entity.insert((
						ElementNode { self_closing },
						ItemOf::<ElementNode, _>::new(file_span),
						ItemOf::<ElementNode, _>::new(node_span),
					));
				}
				let entity = entity.id();

				open_tag
					.attributes
					.into_iter()
					.for_each(|attr| self.spawn_attribute(entity, attr));
			}
			Node::Custom(_) => {
				self.diagnostics.push(Diagnostic::spanned(
					node.span(),
					Level::Error,
					"Unhandled custom node",
				));
			}
		};
	}

	/// Spawn an attribute for the given parent
	fn spawn_attribute(&mut self, parent: Entity, attr: NodeAttribute) {
		match attr {
			NodeAttribute::Block(NodeBlock::Invalid(block)) => {
				// invalid
				self.diagnostics.push(Diagnostic::spanned(
					block.span(),
					Level::Error,
					"Invalid block",
				));
			}
			NodeAttribute::Block(NodeBlock::ValidBlock(block)) => {
				let block_file_span =
					FileSpan::new_from_span(self.file.clone(), &block);
				let block_span = SendWrapper::new(block.span());
				// block attribute, ie `<div {is_hidden}>`
				self.commands.spawn((
					AttributeOf::new(parent),
					ItemOf::<AttributeExpr, _>::new(block_file_span),
					ItemOf::<AttributeExpr, _>::new(block_span),
					AttributeExpr::new(syn::parse_quote!(
						#[allow(unused_braces)]
						#block
					)),
				));
			}
			NodeAttribute::Attribute(attr) => {
				let key = match &attr.key {
					NodeName::Block(block) => {
						self.diagnostics.push(Diagnostic::spanned(
							block.span(),
							Level::Error,
							"Block tag names are not supported as attribute keys",
						));
						"block-key".to_string()
					}
					key => key.to_string(),
				};
				let mut entity = self.commands.spawn((
					AttributeOf::new(parent),
					AttributeKey::new(key),
					ItemOf::<AttributeKey, _>::new(FileSpan::new_from_span(
						self.file.clone(),
						&attr.key,
					)),
					ItemOf::<AttributeKey, _>::new(SendWrapper::new(
						attr.key.span(),
					)),
				));
				// key-value attribute, ie `<div hidden=true>`
				let val_expr_span =
					SendWrapper::new(attr.possible_value.span());
				let val_expr_file_span = FileSpan::new_from_span(
					self.file.clone(),
					&attr.possible_value,
				);

				match attr.possible_value {
					KeyedAttributeValue::Value(value) => match value.value {
						KVAttributeValue::Expr(mut val_expr) => {
							if let Expr::Lit(ExprLit { lit, attrs: _ }) =
								&val_expr
							{
								entity.insert(lit_to_attr(lit));
							}
							if let Expr::Block(block) = &val_expr {
								val_expr = syn::parse_quote!(
									#[allow(unused_braces)]
									#block
								);
							}
							entity.insert((
								AttributeExpr::new(val_expr),
								ItemOf::<AttributeExpr, _>::new(
									val_expr_file_span,
								),
								ItemOf::<AttributeExpr, _>::new(val_expr_span),
							));
						}
						KVAttributeValue::InvalidBraced(invalid) => {
							// invalid
							self.diagnostics.push(Diagnostic::spanned(
								invalid.span(),
								Level::Error,
								"Invalid block",
							));
						}
					},
					_ => {
						// key-only attribute, ie `<div hidden>`
					}
				};
			}
		}
	}

	/// Simplifies parsing an `rstml::NodeName`
	fn parse_node_name(
		&mut self,
		name: &NodeName,
	) -> (String, FileSpan, SendWrapper<Span>) {
		match name {
			NodeName::Block(block) => {
				self.diagnostics.push(Diagnostic::spanned(
					block.span(),
					Level::Error,
					"Block tag names are not supported",
				));
			}
			_ => {}
		}
		let key_str = name.to_string();
		(
			key_str,
			FileSpan::new_from_span(self.file.clone(), name),
			SendWrapper::new(name.span()),
		)
	}

	/// Ensure that self-closing elements do not have children,
	/// ie <br>foo</br>
	fn check_self_closing_children(
		&mut self,
		element: &NodeElement<RstmlCustomNode>,
	) {
		if element.children.is_empty()
			|| !self
				.rstml_config
				.self_closing_elements
				.contains(element.open_tag.name.to_string().as_str())
		{
			return;
		}
		self.diagnostics.push(Diagnostic::spanned(
			element.open_tag.name.span(),
			Level::Warning,
			"Self closing elements cannot have children",
		));
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_bevy::prelude::*;
	use beet_common::prelude::*;
	use beet_utils::prelude::*;
	use bevy::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;


	fn parse(tokens: TokenStream) -> App {
		App::new()
			.add_plugins((tokens_to_rstml_plugin, rstml_to_node_tokens_plugin))
			.xtap(|app| {
				app.world_mut().spawn((
					SourceFile::new(WsPathBuf::new(file!())),
					RstmlTokens::new(tokens),
				));
			})
			.update_then()
			.xmap(std::mem::take)
	}


	#[test]
	fn works() {
		let mut app = quote! {
			<span>
				<MyComponent client:load />
				<div/>
			</span>
		}
		.xmap(parse);
		app.query_once::<&NodeTag>().xpect().to_have_length(3);

		app.query_once::<&AttributeKey>()[0]
			.xmap(|attr| attr.clone().0)
			.xpect()
			.to_be("client:load");
	}
}

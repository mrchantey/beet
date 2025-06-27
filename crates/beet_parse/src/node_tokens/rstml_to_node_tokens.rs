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
	mut query: Populated<
		(Entity, &RstmlRoot, &MacroIdx, &mut TokensDiagnostics),
		Added<RstmlRoot>,
	>,
) -> Result {
	for (entity, rstml_nodes, macro_idx, diagnostics) in query.iter_mut() {
		let root_node = rstml_nodes.clone();

		let mut collected_elements = CollectedElements::default();

		RstmlToWorld {
			file_path: &macro_idx.file,
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
	file_path: &'a WsPathBuf,
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
		let node_span = node.span();
		let file_span = FileSpan::new_from_span(self.file_path.clone(), &node);
		// let spans = (node_span, file_span);
		match node {
			Node::Doctype(_) => {
				self.commands.entity(entity).insert((
					DoctypeNode,
					FileSpanOf::<DoctypeNode>::new(file_span),
					SpanOf::<DoctypeNode>::new(node_span),
				));
			}
			Node::Comment(node) => {
				self.commands.entity(entity).insert((
					CommentNode(node.value.value()),
					FileSpanOf::<CommentNode>::new(file_span),
					SpanOf::<CommentNode>::new(node_span),
				));
			}
			Node::Text(node) => {
				self.commands.entity(entity).insert((
					TextNode(node.value.value()),
					FileSpanOf::<TextNode>::new(file_span),
					SpanOf::<TextNode>::new(node_span),
				));
			}
			Node::RawText(node) => {
				self.commands.entity(entity).insert((
					TextNode(node.to_string_best()),
					FileSpanOf::<TextNode>::new(file_span),
					SpanOf::<TextNode>::new(node_span),
				));
			}
			Node::Fragment(fragment) => {
				let children = self.spawn_nodes(fragment.children);
				self.commands
					.entity(entity)
					.insert((
						FragmentNode,
						FileSpanOf::<FragmentNode>::new(file_span),
						SpanOf::<FragmentNode>::new(node_span),
					))
					.add_children(&children);
			}
			Node::Block(NodeBlock::ValidBlock(block)) => {
				self.commands.entity(entity).insert((
					BlockNode,
					self.expr_idx.next(),
					FileSpanOf::<BlockNode>::new(file_span),
					SpanOf::<BlockNode>::new(node_span),
					NodeExpr::new_block(block),
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
					FileSpanOf::<BlockNode>::new(file_span),
					SpanOf::<BlockNode>::new(node_span),
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

				self.collected_elements.push((
					tag_str.clone(),
					send_wrapper::SendWrapper::new(tag_span.clone()),
				));
				let self_closing = close_tag.is_none();
				if let Some(close_tag) = close_tag.as_ref() {
					let (close_tag_name, _, close_tag_span) =
						self.parse_node_name(&close_tag.name);
					self.collected_elements.push((
						close_tag_name,
						send_wrapper::SendWrapper::new(close_tag_span),
					));
				}

				let mut entity = self.commands.entity(entity);
				entity.insert((
					NodeTag(tag_str.clone()),
					FileSpanOf::<NodeTag>::new(tag_file_span),
					SpanOf::<NodeTag>::new(tag_span),
				));

				if tag_str.starts_with(|c: char| c.is_uppercase()) {
					entity.insert((
						TemplateNode,
						self.expr_idx.next(),
						FileSpanOf::<TemplateNode>::new(file_span),
						SpanOf::<TemplateNode>::new(node_span),
					));
				} else {
					entity.insert((
						ElementNode { self_closing },
						FileSpanOf::<ElementNode>::new(file_span),
						SpanOf::<ElementNode>::new(node_span),
					));
				}
				let entity = entity.id();

				open_tag
					.attributes
					.into_iter()
					.for_each(|attr| self.spawn_attribute(entity, attr));

				let children = self.spawn_nodes(children);
				self.commands.entity(entity).add_children(&children);
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
					FileSpan::new_from_span(self.file_path.clone(), &block);
				// block attribute, ie `<div {is_hidden}>`
				self.commands.spawn((
					AttributeOf::new(parent),
					FileSpanOf::<NodeExpr>::new(block_file_span),
					SpanOf::<NodeExpr>::new(block.span()),
					NodeExpr::new_block(block),
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
					FileSpanOf::<AttributeKey>::new(FileSpan::new_from_span(
						self.file_path.clone(),
						&attr.key,
					)),
					SpanOf::<AttributeKey>::new(attr.key.span()),
				));
				// key-value attribute, ie `<div hidden=true>`
				let val_expr_file_span = FileSpan::new_from_span(
					self.file_path.clone(),
					&attr.possible_value,
				);

				let val_expr_span = attr.possible_value.span();
				match attr.possible_value {
					KeyedAttributeValue::Value(value) => match value.value {
						KVAttributeValue::Expr(val_expr) => {
							if let Expr::Lit(ExprLit { lit, attrs: _ }) =
								&val_expr
							{
								entity.insert(lit_to_attr(lit));
							} else {
								// non-literal expression, needs an ExprIdx
								entity.insert(self.expr_idx.next());
							}
							entity.insert((
								NodeExpr::new(val_expr),
								FileSpanOf::<NodeExpr>::new(val_expr_file_span),
								SpanOf::<NodeExpr>::new(val_expr_span),
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
	fn parse_node_name(&mut self, name: &NodeName) -> (String, FileSpan, Span) {
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
			FileSpan::new_from_span(self.file_path.clone(), name),
			name.span(),
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
				app.world_mut().spawn(RstmlTokens::new(tokens));
			})
			.update_then()
			.xmap(std::mem::take)
	}


	#[test]
	fn works() {
		let mut app = parse(quote! {
			<span>
				<MyComponent client:load />
				<div/>
			</span>
		});
		app.query_once::<&NodeTag>().xpect().to_have_length(3);

		app.query_once::<&AttributeKey>()[0]
			.xmap(|attr| attr.clone().0)
			.xpect()
			.to_be("client:load");
	}
	#[test]
	fn attribute_expr() {
		let mut app = parse(quote! {<div foo={7} bar="baz"/>});
		app.query_once::<&ExprIdx>().len().xpect().to_be(1);
	}
}

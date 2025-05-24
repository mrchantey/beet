use super::RstmlCustomNode;
use crate::prelude::*;
use beet_common::prelude::*;
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
use sweet::prelude::WorkspacePathBuf;
use syn::Expr;
use syn::ExprBlock;
use syn::ExprLit;
use syn::Lit;
use syn::LitStr;
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
		&RstmlNodes,
		&mut TokensDiagnostics,
	)>,
) -> Result {
	for (entity, source_file, rstml_nodes, diagnostics) in query.iter_mut() {
		let rstml_nodes = rstml_nodes.clone();

		let mut collected_elements = CollectedElements::default();

		RstmlToWorld {
			file: source_file,
			rstml_config: &rstml_config,
			collected_elements: &mut collected_elements,
			diagnostics,
			commands: &mut commands,
			rusty_tracker: Default::default(),
		}
		.map_to_children(entity, rstml_nodes);
		commands
			.entity(entity)
			.remove::<RstmlNodes>()
			.insert(collected_elements);
	}
	Ok(())
}


struct RstmlToWorld<'w, 's, 'a> {
	file: &'a WorkspacePathBuf,
	rstml_config: &'a RstmlConfig,
	collected_elements: &'a mut CollectedElements,
	diagnostics: Mut<'a, TokensDiagnostics>,
	commands: &'a mut Commands<'w, 's>,
	rusty_tracker: RustyTrackerBuilder,
}

impl<'w, 's, 'a> RstmlToWorld<'w, 's, 'a> {
	/// Parse all nodes in [`RstmlNodes`] and add their tokens as children
	fn map_to_children(&mut self, root: Entity, nodes: RstmlNodes) {
		let span = if nodes.len() == 1 {
			nodes.first().unwrap().span()
		} else {
			nodes
				.first()
				.map(|n| n.span())
				.unwrap_or(Span::call_site())
				.join(
					nodes.last().map(|n| n.span()).unwrap_or(Span::call_site()),
				)
				.unwrap_or(Span::call_site())
		};
		let children = self.map_nodes(nodes.take());
		self.commands
			.entity(root)
			.insert(ItemOf::<(), _>::new(FileSpan::new_from_span(
				self.file.clone(),
				&span,
			)))
			.add_children(&children);
	}

	/// the number of actual html nodes will likely be different
	/// due to fragments, blocks etc.
	/// Returns the entity containing these nodes
	pub fn map_nodes(
		&mut self,
		nodes: Vec<Node<RstmlCustomNode>>,
	) -> Vec<Entity> {
		nodes.into_iter().map(|node| self.map_node(node)).collect()
	}


	fn map_node(&mut self, node: Node<RstmlCustomNode>) -> Entity {
		let node_span = SendWrapper::new(node.span());
		let file_span = FileSpan::new_from_span(self.file.clone(), &node);
		// let spans = (node_span, file_span);
		match node {
			Node::Doctype(_) => self
				.commands
				.spawn((
					DoctypeNode,
					ItemOf::<DoctypeNode, _>::new(file_span),
					ItemOf::<DoctypeNode, _>::new(node_span),
				))
				.id(),
			Node::Comment(node) => self
				.commands
				.spawn((
					CommentNode(node.value.value()),
					ItemOf::<CommentNode, _>::new(file_span),
					ItemOf::<CommentNode, _>::new(node_span),
				))
				.id(),
			Node::Text(node) => self
				.commands
				.spawn((
					TextNode(node.value.value()),
					ItemOf::<TextNode, _>::new(file_span),
					ItemOf::<TextNode, _>::new(node_span),
				))
				.id(),
			Node::RawText(node) => self
				.commands
				.spawn((
					TextNode(node.to_string_best()),
					ItemOf::<TextNode, _>::new(file_span),
					ItemOf::<TextNode, _>::new(node_span),
				))
				.id(),
			Node::Fragment(fragment) => {
				let children = self.map_nodes(fragment.children);
				self.commands
					.spawn((
						FragmentNode,
						ItemOf::<FragmentNode, _>::new(file_span),
						ItemOf::<FragmentNode, _>::new(node_span),
					))
					.add_children(&children)
					.id()
			}
			Node::Block(NodeBlock::ValidBlock(block)) => {
				let tracker = self.rusty_tracker.next_tracker(&block);
				let expr = SendWrapper::new(Expr::Block(ExprBlock {
					attrs: Vec::new(),
					label: None,
					block,
				}));
				self.commands
					.spawn((
						BlockNode,
						ItemOf::<BlockNode, _>::new(file_span),
						ItemOf::<BlockNode, _>::new(node_span),
						ItemOf::<BlockNode, _>::new(tracker),
						ItemOf::<BlockNode, _>::new(expr),
					))
					.id()
			}
			Node::Block(NodeBlock::Invalid(invalid)) => {
				self.diagnostics.push(Diagnostic::spanned(
					invalid.span(),
					Level::Error,
					"Invalid block",
				));
				self.commands
					.spawn((
						BlockNode,
						ItemOf::<BlockNode, _>::new(file_span),
						ItemOf::<BlockNode, _>::new(node_span),
					))
					.id()
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

				let children = self.map_nodes(children);

				let mut entity = self.commands.spawn((
					NodeTag(tag_str.clone()),
					ItemOf::<NodeTag, _>::new(tag_file_span),
					ItemOf::<NodeTag, _>::new(tag_span),
				));
				entity.add_children(&children);

				if tag_str.starts_with(|c: char| c.is_uppercase()) {
					// yes we get the tracker after its children, its fine as long
					// as its consistent with other parsers.
					let tracker =
						self.rusty_tracker.next_from_open_tag(&open_tag);
					entity.insert((
						TemplateNode,
						ItemOf::<TemplateNode, _>::new(tracker),
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


				entity
			}
			Node::Custom(_) => {
				self.diagnostics.push(Diagnostic::spanned(
					node.span(),
					Level::Error,
					"Unhandled custom node",
				));
				Entity::PLACEHOLDER
			}
		}
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
					AttributeExpr::new(Expr::Block(ExprBlock {
						attrs: Vec::new(),
						label: None,
						block,
					})),
				));
			}
			NodeAttribute::Attribute(attr) => {
				let key_expr = self.node_name_to_expr(attr.key);
				let key_expr_span = SendWrapper::new(key_expr.span());
				let key_expr_file_span =
					FileSpan::new_from_span(self.file.clone(), &key_expr);

				let mut entity = self.commands.spawn((
					AttributeOf::new(parent),
					ItemOf::<AttributeKeyExpr, _>::new(
						key_expr_file_span.clone(),
					),
					ItemOf::<AttributeKeyExpr, _>::new(key_expr_span),
				));


				let key_lit =
					if let Expr::Lit(ExprLit { lit, attrs: _ }) = &key_expr {
						Some(lit_to_string(lit))
					} else {
						None
					};
				entity.insert(AttributeKeyExpr::new(key_expr));

				let mut val_lit = None;

				match attr.possible_value {
					KeyedAttributeValue::Value(value) => match value.value {
						KVAttributeValue::Expr(val_expr) => {
							// key-value attribute, ie `<div hidden=true>`
							let val_expr_span =
								SendWrapper::new(val_expr.span());
							let val_expr_file_span = FileSpan::new_from_span(
								self.file.clone(),
								&val_expr,
							);
							if let Expr::Lit(ExprLit { lit, attrs: _ }) =
								&val_expr
							{
								val_lit = Some(lit_to_string(lit));
							}

							entity.insert((
								AttributeValueExpr::new(val_expr),
								ItemOf::<AttributeValueExpr, _>::new(
									val_expr_file_span,
								),
								ItemOf::<AttributeValueExpr, _>::new(
									val_expr_span,
								),
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

				if let Some(key_lit) = key_lit {
					entity.insert(AttributeLit::new(key_lit, val_lit));
				}
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
	fn node_name_to_expr(&mut self, name: NodeName) -> Expr {
		match name {
			NodeName::Block(block) => Expr::Block(ExprBlock {
				attrs: Vec::new(),
				label: None,
				block,
			}),
			name => Expr::Lit(ExprLit {
				lit: Lit::Str(LitStr::new(&name.to_string(), name.span())),
				attrs: Vec::new(),
			}),
		}
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
	use beet_common::prelude::*;
	use bevy::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;

	fn parse(tokens: TokenStream) -> App {
		App::new()
			.add_plugins((tokens_to_rstml_plugin, rstml_to_node_tokens_plugin))
			.xtap(|app| {
				app.world_mut().spawn((
					SourceFile::new(WorkspacePathBuf::new(file!())),
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

		app.query_once::<&AttributeLit>()[0]
			.xmap(|attr| attr.key.clone())
			.xpect()
			.to_be("client:load");
	}
}

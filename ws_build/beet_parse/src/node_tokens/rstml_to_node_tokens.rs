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
use sweet::prelude::WorkspacePathBuf;
use syn::Block;
use syn::Expr;
use syn::Lit;
use syn::spanned::Spanned;

use super::RstmlCustomNode;


pub fn rstml_to_node_tokens_plugin(app: &mut App) {
	app.init_non_send_resource::<NonSendAssets<Span>>()
		.init_non_send_resource::<NonSendAssets<Expr>>()
		.init_non_send_resource::<NonSendAssets<Lit>>()
		.init_non_send_resource::<NonSendAssets<Block>>()
		.init_non_send_resource::<NonSendAssets<CollectedElements>>()
		.add_systems(
			Update,
			rstml_to_node_tokens.after(super::tokens_to_rstml),
		);
}

/// Replace [`RstmlNodes`] with children representing each [`rstml::Node`].
fn rstml_to_node_tokens(
	mut commands: Commands,
	rstml_config: Res<RstmlConfig>,
	mut spans_map: NonSendMut<NonSendAssets<Span>>,
	mut expr_map: NonSendMut<NonSendAssets<Expr>>,
	mut lit_map: NonSendMut<NonSendAssets<Lit>>,
	mut block_map: NonSendMut<NonSendAssets<Block>>,
	mut diagnostics_map: NonSendMut<NonSendAssets<TokensDiagnostics>>,
	mut nodes_map: NonSendMut<NonSendAssets<RstmlNodes>>,
	mut collected_elements_map: NonSendMut<NonSendAssets<CollectedElements>>,
	query: Populated<(
		Entity,
		&SourceFile,
		&NonSendHandle<RstmlNodes>,
		&NonSendHandle<TokensDiagnostics>,
	)>,
) -> Result {
	for (entity, source_file, rstml_nodes, diagnostics) in query.iter() {
		let rstml_nodes = nodes_map.remove(rstml_nodes)?;
		let diagnostics = diagnostics_map.get_mut(diagnostics)?;

		let mut collected_elements = CollectedElements::default();

		RstmlToWorld {
			file: source_file,
			rstml_config: &rstml_config,
			collected_elements: &mut collected_elements,
			diagnostics,
			commands: &mut commands,
			rusty_tracker: Default::default(),
			spans_map: &mut spans_map,
			expr_map: &mut expr_map,
			lit_map: &mut lit_map,
			block_map: &mut block_map,
		}
		.insert(entity, rstml_nodes);
		commands
			.entity(entity)
			.remove::<NonSendHandle<RstmlNodes>>()
			.insert(collected_elements_map.insert(collected_elements));
	}
	Ok(())
}


struct RstmlToWorld<'w, 's, 'a> {
	file: &'a WorkspacePathBuf,
	rstml_config: &'a RstmlConfig,
	collected_elements: &'a mut CollectedElements,
	diagnostics: &'a mut TokensDiagnostics,
	commands: &'a mut Commands<'w, 's>,
	rusty_tracker: RustyTrackerBuilder,
	spans_map: &'a mut NonSendAssets<Span>,
	expr_map: &'a mut NonSendAssets<Expr>,
	lit_map: &'a mut NonSendAssets<Lit>,
	block_map: &'a mut NonSendAssets<Block>,
}

impl<'w, 's, 'a> RstmlToWorld<'w, 's, 'a> {
	/// Parse all nodes in [`RstmlNodes`] and add their tokens as children
	fn insert(&mut self, root: Entity, nodes: RstmlNodes) {
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
		let children = self.map_nodes(nodes.into_inner());
		self.commands
			.entity(root)
			.insert(FileSpan::new_from_span(self.file.clone(), &span))
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
		let node_span = self.spans_map.insert(node.span());
		let file_span = FileSpan::new_from_span(self.file.clone(), &node);
		let spans = (node_span, file_span);
		match node {
			Node::Doctype(_) => self.commands.spawn((DoctypeNode, spans)).id(),
			Node::Comment(node) => self
				.commands
				.spawn((
					CommentNode {
						value: node.value.value(),
					},
					spans,
				))
				.id(),
			Node::Text(node) => self
				.commands
				.spawn((
					TextNode {
						value: node.value.value(),
					},
					spans,
				))
				.id(),
			Node::RawText(node) => self
				.commands
				.spawn((
					TextNode {
						value: node.to_string_best(),
					},
					spans,
				))
				.id(),
			Node::Fragment(fragment) => {
				let children = self.map_nodes(fragment.children);
				self.commands
					.spawn((FragmentNode, spans))
					.add_children(&children)
					.id()
			}
			Node::Block(NodeBlock::ValidBlock(block)) => {
				let tracker = self.rusty_tracker.next_tracker(&block);
				let handle = self.block_map.insert(block);
				self.commands
					.spawn((BlockNode { tracker, handle }, spans))
					.id()
			}
			Node::Block(NodeBlock::Invalid(invalid)) => {
				self.diagnostics.push(Diagnostic::spanned(
					invalid.span(),
					Level::Error,
					"Invalid block",
				));
				Entity::PLACEHOLDER
			}
			Node::Element(el) => {
				self.check_self_closing_children(&el);

				let NodeElement {
					open_tag,
					children,
					close_tag,
				} = el;
				let (tag, tag_span) = self.map_node_name(&open_tag.name);

				self.collected_elements.push((tag.clone(), tag_span));
				let self_closing = close_tag.is_none();
				if let Some(close_tag) = close_tag.as_ref() {
					let close_tag = self.map_node_name(&close_tag.name);
					self.collected_elements.push(close_tag);
				}
				open_tag.span();
				let attributes = open_tag
					.attributes
					.clone()
					.into_iter()
					.map(|attr| self.map_attribute(attr))
					.collect::<Vec<_>>();
				let attributes = AttributeTokensList(attributes);

				let children = self.map_nodes(children);

				if tag.starts_with(|c: char| c.is_uppercase()) {
					// dont hash the span
					// at this stage directives are still attributes, which
					// is good because we want to hash those too
					let tracker = self.rusty_tracker.next_tracker(&attributes);

					self.commands
						.spawn((
							ComponentNode {
								tag,
								tag_span: Some(tag_span),
								tracker,
							},
							spans,
						))
						.add_children(&children)
						.id()
				} else {
					self.commands
						.spawn((ElementNode { tag, self_closing }, spans))
						.add_children(&children)
						.id()
				}
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

	fn map_attribute(&mut self, attr: NodeAttribute) -> AttributeTokens {
		match attr {
			NodeAttribute::Block(NodeBlock::Invalid(block)) => {
				self.diagnostics.push(Diagnostic::spanned(
					block.span(),
					Level::Error,
					"Invalid block",
				));
				AttributeTokens::Key {
					key: Spanner::new(
						FileSpan::new_from_span(self.file.clone(), &block),
						"invalid-block".to_string(),
					),
					key_span: None,
				}
			}
			NodeAttribute::Block(NodeBlock::ValidBlock(block)) => {
				AttributeTokens::Block {
					tracker: self.rusty_tracker.next_tracker(&block),
					value: Spanner::new(
						FileSpan::new_from_span(self.file.clone(), &block),
						self.block_map.insert(block),
					),
				}
			}
			NodeAttribute::Attribute(attr) => {
				// let value = attr.value().cloned();
				let (key, key_span) = self.map_node_name(&attr.key);
				let value = if let KeyedAttributeValue::Value(expr) =
					attr.possible_value
					&& let KVAttributeValue::Expr(expr) = expr.value
				{
					Some(expr)
				} else {
					None
				};


				match value {
					// lit expr
					Some(Expr::Lit(expr)) => AttributeTokens::KeyValueLit {
						key,
						key_span: Some(key_span),
						value: Spanner::new(
							FileSpan::new_from_span(self.file.clone(), &expr),
							self.lit_map.insert(expr.lit),
						),
					},
					// non-lit expr
					Some(expr) => AttributeTokens::KeyValueExpr {
						key,
						key_span: Some(key_span),
						tracker: self.rusty_tracker.next_tracker(&expr),
						value: Spanner::new(
							FileSpan::new_from_span(self.file.clone(), &expr),
							self.expr_map.insert(expr),
						),
					},
					// no value expr
					None => AttributeTokens::Key {
						key,
						key_span: Some(key_span),
					},
				}
			}
		}
	}
	/// Simplifies the [`NodeName::Punctuated`],ie client:load to a string literal
	fn map_node_name(
		&mut self,
		name: &NodeName,
	) -> (Spanner<String>, NonSendHandle<Span>) {
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
			Spanner::new(
				FileSpan::new_from_span(self.file.clone(), name),
				key_str,
			),
			self.spans_map.insert(name.span()),
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
	use beet_common::prelude::*;
	use bevy::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;

	// fn map(tokens: TokenStream) -> WebTokens {
	// 	tokens
	// 		.xpipe(TokensToRstml::default())
	// 		.0
	// 		.xpipe(RstmlToWebTokens::default())
	// 		.0
	// 		.xpipe(ParseWebTokens::default())
	// }

	fn map(tokens: TokenStream) -> App {
		App::new()
			.add_plugins((tokens_to_rstml_plugin, rstml_to_node_tokens_plugin))
			.xtap(|app| {
				app.world_mut()
					.spawn(SourceFile::new(WorkspacePathBuf::new(file!())))
					.insert_non_send(RstmlTokens::new(tokens));
			})
			.update_then()
			.xmap(std::mem::take)
	}


	#[test]
	fn works() {
		quote! {
			<span>
				<MyComponent client:load />
				<div/>
			</span>
		}
		.xmap(map)
		.query_once::<&FileSpan>()
		.xpect()
		.to_have_length(4);
	}
}

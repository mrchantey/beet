use crate::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use proc_macro2_diagnostics::Level;
use quote::quote;
use rstml::node::Node;
use rstml::node::NodeAttribute;
use rstml::node::NodeBlock;
use rstml::node::NodeElement;
use rstml::node::NodeFragment;
use rstml::node::NodeName;
use std::collections::HashSet;
use std::ops::ControlFlow;
use sweet::prelude::PipelineTarget;
use syn::spanned::Spanned;



/// Convert rstml nodes to a Vec<RsxNode> token stream
#[derive(Debug, Default)]
pub struct RstmlToNodeTokens<C: CustomNodeTokens = ()> {
	custom_parser: C::RstmlParser,
	pub idents: RsxIdents,
	// Additional error and warning messages.
	pub errors: Vec<TokenStream>,
	// Collect elements to provide semantic highlight based on element tag.
	// No differences between open tag and closed tag.
	// Also multiple tags with same name can be present,
	// because we need to mark each of them.
	pub collected_elements: Vec<NodeName>,
	// rstml requires std hashset :(
	pub self_closing_elements: HashSet<&'static str>,
}

impl<C: CustomNodeTokens> RstmlToNodeTokens<C>
where
	C::RstmlParser: std::default::Default,
{
	pub fn new(idents: RsxIdents) -> Self {
		Self {
			idents,
			custom_parser: Default::default(),
			errors: Default::default(),
			collected_elements: Default::default(),
			self_closing_elements: self_closing_elements(),
		}
	}
}
impl<C: CustomNodeTokens> RstmlToNodeTokens<C> {
	/// Parse rstml tokens to a [`NodeTokens`] and any compile errors
	pub fn parse(
		&mut self,
		tokens: TokenStream,
	) -> (NodeTokens<C>, TokenStream) {
		let (nodes, rstml_errors) =
			tokens.xpipe(TokensToRstml::<C::CustomRstmlNode>::default());
		let node = self.map_nodes(nodes);

		let rstml_to_rsx_errors = &self.errors;
		(node, quote! {
			{
				#(#rstml_errors;)*
				#(#rstml_to_rsx_errors;)*
			}
		})
	}

	/// the number of actual html nodes will likely be different
	/// due to fragments, blocks etc
	pub fn map_nodes(
		&mut self,
		nodes: Vec<Node<C::CustomRstmlNode>>,
	) -> NodeTokens<C> {
		let mut nodes = nodes
			.into_iter()
			.map(|node| self.map_node(node))
			.collect::<Vec<_>>();
		if nodes.len() == 1 {
			nodes.pop().unwrap()
		} else {
			NodeTokens::Fragment {
				nodes,
				directives: Default::default(),
			}
		}
	}

	/// returns an RsxNode
	fn map_node(&mut self, node: Node<C::CustomRstmlNode>) -> NodeTokens<C> {
		let node = match self.custom_parser.map_node(node) {
			ControlFlow::Continue(rstml_node) => rstml_node,
			ControlFlow::Break(rsx_node) => {
				return rsx_node;
			}
		};

		match node {
			Node::Text(text) => NodeTokens::Text {
				text: text.value_string(),
				directives: Default::default(),
			},
			Node::RawText(raw) => NodeTokens::Text {
				text: raw.to_string_best(),
				directives: Default::default(),
			},
			Node::Fragment(NodeFragment { children, .. }) => {
				NodeTokens::Fragment {
					nodes: children
						.into_iter()
						.map(|n| self.map_node(n))
						.collect(),
					// rstml <> fragments dont allow attributes
					directives: Default::default(),
				}
			}
			Node::Block(NodeBlock::ValidBlock(block)) => NodeTokens::Block {
				block: Spanner::new_spanned(block),
				directives: Default::default(),
			},
			Node::Block(NodeBlock::Invalid(invalid)) => {
				self.errors.push(
					Diagnostic::spanned(
						invalid.span(),
						Level::Error,
						"Invalid block",
					)
					.emit_as_expr_tokens(),
				);
				Default::default()
			}
			Node::Element(el) => {
				self.check_self_closing_children(&el);

				let NodeElement {
					open_tag,
					children,
					close_tag,
				} = el;


				let (mut directives, attributes) =
					MetaBuilder::parse_attributes(open_tag.attributes);



				self.collected_elements.push(open_tag.name.clone());
				if let Some(close_tag) = close_tag {
					self.collected_elements.push(close_tag.name.clone());
					directives.push(TemplateDirectiveTokens::CustomKey(
						NameExpr::String(Spanner::new_custom_spanned(
							"self-closing",
							&close_tag,
						)),
					))
				}

				// TODO check this after into NodeTokens
				#[cfg(feature = "css")]
				if open_tag.name.to_string() == "style" {
					if let Err(err) = validate_style_node(&children) {
						self.errors.push(
							Diagnostic::spanned(err.0, Level::Error, err.1)
								.emit_as_expr_tokens(),
						);
					}
				}

				let attributes = attributes
					.into_iter()
					.filter_map(|attr| self.map_attribute(attr))
					.collect::<Vec<_>>();

				NodeTokens::Component {
					tag: open_tag.name.into(),
					attributes,
					directives,
					children: Box::new(self.map_nodes(children)),
				}
			}
			Node::Custom(_) => {
				self.errors.push(
					Diagnostic::spanned(
						node.span(),
						Level::Error,
						"Unhandled custom node",
					)
					.emit_as_expr_tokens(),
				);
				Default::default()
			}
			_ => {
				self.errors.push(
					Diagnostic::spanned(
						node.span(),
						Level::Error,
						"Unhandled node",
					)
					.emit_as_expr_tokens(),
				);
				Default::default()
			}
		}
	}

	fn map_attribute(
		&mut self,
		attr: NodeAttribute,
	) -> Option<RsxAttributeTokens> {
		match attr {
			NodeAttribute::Block(NodeBlock::ValidBlock(block)) => {
				Some(RsxAttributeTokens::Block {
					block: block.into(),
				})
			}
			NodeAttribute::Block(NodeBlock::Invalid(_)) => {
				self.errors.push(
					Diagnostic::spanned(
						attr.span(),
						Level::Error,
						"Invalid block",
					)
					.emit_as_expr_tokens(),
				);
				None
			}
			NodeAttribute::Attribute(attr) => {
				let value = attr.value().cloned();
				match (attr.key, value) {
					(key, Some(value)) => Some(RsxAttributeTokens::KeyValue {
						key: key.into(),
						value: value.into(),
					}),
					(key, None) => {
						Some(RsxAttributeTokens::Key { key: key.into() })
					}
				}
			}
		}
	}

	/// Ensure that self-closing elements do not have children.
	fn check_self_closing_children(
		&mut self,
		element: &NodeElement<C::CustomRstmlNode>,
	) {
		if element.children.is_empty()
			|| !self
				.self_closing_elements
				.contains(element.open_tag.name.to_string().as_str())
		{
			return;
		}
		let warning = Diagnostic::spanned(
			element.open_tag.name.span(),
			Level::Warning,
			"Element is processed as empty, and cannot have any child",
		);
		self.errors.push(warning.emit_as_expr_tokens());
	}


	/// Update [`Self::idents`] with the specified runtime and removes it from
	/// the list of attributes. See [`RsxIdents::set_runtime`] for more information.
	#[allow(unused)]
	fn parse_runtime_directive(
		&mut self,
		directives: &[TemplateDirectiveTokens],
	) {
		for directive in directives.iter() {
			if let TemplateDirectiveTokens::Runtime(runtime) = directive {
				if let Err(err) = self.idents.runtime.set(runtime) {
					let diagnostic = Diagnostic::spanned(
						Span::call_site(),
						Level::Error,
						err.to_string(),
					);
					self.errors.push(diagnostic.emit_as_expr_tokens());
				}
			}
		}
	}
}

#[cfg(test)]
mod test {

	// #[test]
	// fn style() { let _block = map(quote! {
	// 	<style>
	// 		main {
	// 			/* min-height:100dvh; */
	// 			min-height: var(--bm-main-height);
	// 			padding: 1em var(--content-padding-width);
	// 		}
	// </style>
	// }); }
}

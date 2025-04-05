use crate::prelude::*;
use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use proc_macro2_diagnostics::Level;
use rstml::node::CustomNode;
use rstml::node::Node;
use rstml::node::NodeAttribute;
use rstml::node::NodeBlock;
use rstml::node::NodeElement;
use rstml::node::NodeFragment;
use rstml::node::NodeName;
use std::collections::HashSet;
use sweet::prelude::Pipeline;
use syn::spanned::Spanned;



/// Convert rstml nodes to a Vec<RsxNode> token stream
#[derive(Debug, Default)]
pub struct RstmlToRsxTokens<C = rstml::Infallible> {
	// Additional error and warning messages.
	pub errors: Vec<TokenStream>,
	// Collect elements to provide semantic highlight based on element tag.
	// No differences between open tag and closed tag.
	// Also multiple tags with same name can be present,
	// because we need to mark each of them.
	pub collected_elements: Vec<NodeName>,
	// rstml requires std hashset :(
	pub self_closing_elements: HashSet<&'static str>,
	phantom: std::marker::PhantomData<C>,
}

impl RstmlToRsxTokens {
	pub fn new() -> Self {
		Self {
			errors: Vec::new(),
			collected_elements: Vec::new(),
			self_closing_elements: self_closing_elements(),
			phantom: std::marker::PhantomData,
		}
	}
}

/// Parse rstml nodes to a [`NodeTokens`] and any compile errors
impl<C: CustomNode> Pipeline<Vec<Node<C>>, (RsxNodeTokens, Vec<TokenStream>)>
	for RstmlToRsxTokens<C>
{
	fn apply(
		mut self,
		nodes: Vec<Node<C>>,
	) -> (RsxNodeTokens, Vec<TokenStream>) {
		let node = self.map_nodes(nodes);
		(node, self.errors)
	}
}

impl<C: CustomNode> RstmlToRsxTokens<C> {
	/// the number of actual html nodes will likely be different
	/// due to fragments, blocks etc
	pub fn map_nodes(&mut self, nodes: Vec<Node<C>>) -> RsxNodeTokens {
		let mut nodes = nodes
			.into_iter()
			.map(|node| self.map_node(node))
			.collect::<Vec<_>>();
		if nodes.len() == 1 {
			nodes.pop().unwrap()
		} else {
			RsxNodeTokens::Fragment {
				nodes,
				directives: Default::default(),
			}
		}
	}

	/// returns an RsxNode
	fn map_node(&mut self, node: Node<C>) -> RsxNodeTokens {
		match node {
			Node::Doctype(node) => RsxNodeTokens::Component {
				tag: NameExpr::string_spanned("doctype", &node),
				attributes: Default::default(),
				directives: Default::default(),
				children: Default::default(),
			},
			Node::Comment(node) => RsxNodeTokens::Component {
				tag: NameExpr::string_spanned("comment", &node),
				attributes: Default::default(),
				directives: Default::default(),
				children: Box::new(RsxNodeTokens::Text {
					text: node.value.value(),
					directives: Default::default(),
				}),
			},
			Node::Text(text) => RsxNodeTokens::Text {
				text: text.value_string(),
				directives: Default::default(),
			},
			Node::RawText(raw) => RsxNodeTokens::Text {
				text: raw.to_string_best(),
				directives: Default::default(),
			},
			Node::Fragment(NodeFragment { children, .. }) => {
				RsxNodeTokens::Fragment {
					nodes: children
						.into_iter()
						.map(|n| self.map_node(n))
						.collect(),
					// rstml <> fragments dont allow attributes
					directives: Default::default(),
				}
			}
			Node::Block(NodeBlock::ValidBlock(block)) => RsxNodeTokens::Block {
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

				RsxNodeTokens::Component {
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
	fn check_self_closing_children(&mut self, element: &NodeElement<C>) {
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
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use quote::quote;
	use sweet::prelude::*;

	#[test]
	fn works() {
		expect(
			quote! {
				<div/>
			}
			.xpipe(TokensToRstml::new())
			.0
			.xpipe(RstmlToRsxTokens::new())
			.0,
		)
		.to_be(RsxNodeTokens::component(NameExpr::ExprPath(
			Spanner::new_spanned(syn::parse_quote!(div)),
		)));
	}
}

use crate::prelude::*;
use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use proc_macro2_diagnostics::Level;
use quote::ToTokens;
use rstml::node::CustomNode;
use rstml::node::Node;
use rstml::node::NodeAttribute;
use rstml::node::NodeBlock;
use rstml::node::NodeElement;
use rstml::node::NodeFragment;
use rstml::node::NodeName;
use std::collections::HashSet;
use sweet::prelude::Pipeline;
use syn::LitStr;
use syn::spanned::Spanned;



/// Convert rstml nodes to a Vec<RsxNode> token stream
/// ## Pipeline
/// [`Pipeline<Vec<Node<C>>, (HtmlTokens, Vec<TokenStream>)>`]
#[derive(Debug, Default)]
pub struct RstmlToHtmlTokens<C = rstml::Infallible> {
	// Additional error and warning messages.
	pub errors: Vec<TokenStream>,
	// Collect elements to provide semantic highlight based on element tag.
	// No differences between open tag and closed tag.
	// Also multiple tags with same name can be present,
	// because we need to mark each of them.
	pub collected_elements: Vec<NodeName>,
	// rstml requires std hashset :(
	self_closing_elements: HashSet<&'static str>,
	phantom: std::marker::PhantomData<C>,
}

impl RstmlToHtmlTokens {
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
impl<C: CustomNode> Pipeline<Vec<Node<C>>, (HtmlTokens, Vec<TokenStream>)>
	for RstmlToHtmlTokens<C>
{
	fn apply(mut self, nodes: Vec<Node<C>>) -> (HtmlTokens, Vec<TokenStream>) {
		let node = self.map_nodes(nodes);
		(node, self.errors)
	}
}

impl<C: CustomNode> RstmlToHtmlTokens<C> {
	/// the number of actual html nodes will likely be different
	/// due to fragments, blocks etc
	pub fn map_nodes(&mut self, nodes: Vec<Node<C>>) -> HtmlTokens {
		let mut nodes = nodes
			.into_iter()
			.map(|node| self.map_node(node))
			.collect::<Vec<_>>();
		if nodes.len() == 1 {
			nodes.pop().unwrap()
		} else {
			HtmlTokens::Fragment { nodes }
		}
	}

	fn map_node(&mut self, node: Node<C>) -> HtmlTokens {
		match node {
			Node::Doctype(node) => HtmlTokens::Doctype {
				value: node.token_start.token_lt.into(),
			},
			Node::Comment(node) => HtmlTokens::Comment {
				value: node.value.into(),
			},
			Node::Text(node) => HtmlTokens::Text {
				value: node.value.into(),
			},
			Node::RawText(node) => HtmlTokens::Text {
				value: LitStr::new(&node.to_string_best(), node.span()).into(),
			},
			Node::Fragment(NodeFragment { children, .. }) => {
				HtmlTokens::Fragment {
					nodes: children
						.into_iter()
						.map(|n| self.map_node(n))
						.collect(),
				}
			}
			Node::Block(NodeBlock::ValidBlock(node)) => {
				HtmlTokens::Block { value: node.into() }
			}
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

				self.collected_elements.push(open_tag.name.clone());
				let self_closing = close_tag.is_none();
				if let Some(close_tag) = close_tag {
					self.collected_elements.push(close_tag.name.clone());
				}

				let attributes = open_tag
					.attributes
					.clone()
					.into_iter()
					.filter_map(|attr| self.map_attribute(attr))
					.collect::<Vec<_>>();

				HtmlTokens::Element {
					self_closing,
					component: RsxNodeTokens {
						tag: self.map_node_name(open_tag.name.clone()),
						tokens: open_tag.to_token_stream(),
						attributes,
						directives: Vec::default(),
					},
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
						key: self.map_node_name(key),
						value: value.into(),
					}),
					(key, None) => Some(RsxAttributeTokens::Key {
						key: self.map_node_name(key),
					}),
				}
			}
		}
	}
	/// Simplifies the [`NodeName::Punctuated`],ie client:load to a string literal
	fn map_node_name(&mut self, name: NodeName) -> NameExpr {
		let name_str = name.to_string();
		match name {
			NodeName::Path(path) => NameExpr::ExprPath(path.into()),
			NodeName::Punctuated(punctuated) => {
				let str = LitStr::new(&name_str, punctuated.span());
				NameExpr::LitStr(str.into())
			}
			NodeName::Block(block) => {
				self.errors.push(
					Diagnostic::spanned(
						block.span(),
						Level::Error,
						"Block names are not supported",
					)
					.emit_as_expr_tokens(),
				);
				NameExpr::LitStr(LitStr::new("error", block.span()).into())
			}
		}
	}

	/// Ensure that self-closing elements do not have children,
	/// ie <br>foo</br>
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
	// use crate::prelude::*;
	// use quote::quote;
	// use sweet::prelude::*;

	#[test]
	fn div() {
		// expect(
		// 	quote! {
		// 		<div/>
		// 	}
		// 	.xpipe(TokensToRstml::new())
		// 	.0
		// 	.xpipe(RstmlToRsxTokens::new())
		// 	.0,
		// )
		// .to_be(RsxNodeTokens::new(NameExpr::ExprPath(
		// 	Spanner::new_spanned(syn::parse_quote!(div)),
		// )));
	}
}

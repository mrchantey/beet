use crate::prelude::*;
use beet_common::prelude::*;
use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use proc_macro2_diagnostics::Level;
use rstml::node::CustomNode;
use rstml::node::Node;
use rstml::node::NodeAttribute;
use rstml::node::NodeBlock;
use rstml::node::NodeElement;
use rstml::node::NodeName;
use std::collections::HashSet;
use sweet::prelude::Pipeline;
use sweet::prelude::WorkspacePathBuf;
use syn::Expr;
use syn::LitStr;
use syn::spanned::Spanned;



/// Convert rstml nodes to a Vec<WebNode> token stream
/// ## Pipeline
/// [`Pipeline<Vec<Node<C>>, (WebTokens, Vec<TokenStream>)>`]
#[derive(Debug)]
pub struct RstmlToWebTokens<C = rstml::Infallible> {
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
	/// The span of the entry node, this will be taken
	/// by the first node visited.
	file: WorkspacePathBuf,
	// Collect elements to provide semantic highlight based on element tag.
	// No differences between open tag and closed tag.
	// Also multiple tags with same name can be present,
	// because we need to mark each of them.
	pub rusty_tracker: RustyTrackerBuilder,
}

impl Default for RstmlToWebTokens<rstml::Infallible> {
	fn default() -> Self {
		Self {
			file: WorkspacePathBuf::default(),
			errors: Vec::new(),
			collected_elements: Vec::new(),
			self_closing_elements: self_closing_elements(),
			phantom: std::marker::PhantomData,
			rusty_tracker: RustyTrackerBuilder::default(),
		}
	}
}

impl RstmlToWebTokens<rstml::Infallible> {
	pub fn new(file: WorkspacePathBuf) -> Self {
		Self {
			file,
			..Default::default()
		}
	}
}

/// Parse rstml nodes to a [`NodeTokens`] and any compile errors
impl<C: CustomNode> Pipeline<Vec<Node<C>>, (WebTokens, Vec<TokenStream>)>
	for RstmlToWebTokens<C>
{
	fn apply(mut self, nodes: Vec<Node<C>>) -> (WebTokens, Vec<TokenStream>) {
		let mut node = self.map_nodes(nodes);
		node.push_directive(TemplateDirective::NodeTemplate);
		(node, self.errors)
	}
}

impl<C: CustomNode> RstmlToWebTokens<C> {
	/// the number of actual html nodes will likely be different
	/// due to fragments, blocks etc
	pub fn map_nodes(&mut self, nodes: Vec<Node<C>>) -> WebTokens {
		let mut nodes = nodes
			.into_iter()
			.map(|node| self.map_node(node))
			.collect::<Vec<_>>();
		if nodes.len() == 1 {
			nodes.pop().unwrap()
		} else {
			let (start, end) = LineCol::iter_to_spans(&nodes);

			WebTokens::Fragment {
				nodes,
				meta: FileSpan::new(self.file.clone(), start, end).into(),
			}
		}
	}

	fn map_node(&mut self, node: Node<C>) -> WebTokens {
		match node {
			Node::Doctype(node) => WebTokens::Doctype {
				meta: FileSpan::new_from_span(self.file.clone(), &node).into(),
				value: node.token_start.token_lt.into(),
			},
			Node::Comment(node) => WebTokens::Comment {
				meta: FileSpan::new_from_span(self.file.clone(), &node).into(),
				value: node.value.into(),
			},
			Node::Text(node) => WebTokens::Text {
				meta: FileSpan::new_from_span(self.file.clone(), &node).into(),
				value: node.value.into(),
			},
			Node::RawText(node) => WebTokens::Text {
				meta: FileSpan::new_from_span(self.file.clone(), &node).into(),
				value: LitStr::new(&node.to_string_best(), node.span()).into(),
			},
			Node::Fragment(fragment) => WebTokens::Fragment {
				meta: FileSpan::new_from_span(self.file.clone(), &fragment)
					.into(),
				nodes: fragment
					.children
					.into_iter()
					.map(|n| self.map_node(n))
					.collect(),
			},
			Node::Block(NodeBlock::ValidBlock(node)) => WebTokens::Block {
				tracker: self.rusty_tracker.next_tracker(&node),
				meta: FileSpan::new_from_span(self.file.clone(), &node).into(),
				value: node.into(),
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
				let meta =
					FileSpan::new_from_span(self.file.clone(), &el).into();

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
				let tag = self.map_node_name(&open_tag.name);
				let attributes = open_tag
					.attributes
					.clone()
					.into_iter()
					.filter_map(|attr| self.map_attribute(attr))
					.collect::<Vec<_>>();

				let children = Box::new(self.map_nodes(children));

				if tag.as_str().starts_with(|c: char| c.is_uppercase()) {
					// dont hash the span
					// at this stage directives are still attributes, which
					// is good because we want to hash those too
					let tracker = self.rusty_tracker.next_tracker(&attributes);

					WebTokens::Component {
						component: ElementTokens {
							tag,
							attributes,
							meta,
						},
						children,
						tracker,
					}
				} else {
					WebTokens::Element {
						self_closing,
						component: ElementTokens {
							tag,
							attributes,
							meta,
						},
						children,
					}
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
					tracker: self.rusty_tracker.next_tracker(&block),
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
				// let value = attr.value().cloned();
				let key = self.map_node_name(&attr.key);
				match attr.value() {
					Some(value) if let Expr::Lit(value) = value => {
						Some(RsxAttributeTokens::KeyValueLit {
							key,
							value: value.lit.clone(),
						})
					}
					Some(value) => Some(RsxAttributeTokens::KeyValueExpr {
						tracker: self.rusty_tracker.next_tracker(&value),
						key,
						value: value.clone().into(),
					}),
					None => Some(RsxAttributeTokens::Key { key }),
				}
			}
		}
	}
	/// Simplifies the [`NodeName::Punctuated`],ie client:load to a string literal
	fn map_node_name(&mut self, name: &NodeName) -> Spanner<String> {
		match name {
			NodeName::Block(block) => {
				self.errors.push(
					Diagnostic::spanned(
						block.span(),
						Level::Error,
						"Block tag names are not supported",
					)
					.emit_as_expr_tokens(),
				);
			}
			_ => {}
		}
		Spanner::new_with_span(name.to_string(), name.span())
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
	use crate::prelude::*;
	use anyhow::Result;
	use beet_common::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;

	fn map(tokens: TokenStream) -> Result<WebTokens> {
		tokens
			.xpipe(TokensToRstml::default())
			.0
			.xpipe(RstmlToWebTokens::default())
			.0
			.xpipe(ParseWebTokens::default())
	}

	#[test]
	fn works() {
		quote! {
			<MyComponent client:load />
		}
		.xmap(map)
		.unwrap()
		.reset_spans_and_trackers()
		.xpect()
		.to_be(WebTokens::Component {
			component: ElementTokens {
				tag: "MyComponent".into(),
				attributes: Vec::new(),
				meta: NodeMeta::default().with_template_directives(vec![
					TemplateDirective::NodeTemplate,
					TemplateDirective::ClientLoad,
				]),
			},
			children: Default::default(),
			tracker: RustyTracker::PLACEHOLDER,
		});
	}
}

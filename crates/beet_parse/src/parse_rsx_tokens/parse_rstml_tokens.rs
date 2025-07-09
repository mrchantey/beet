use crate::prelude::*;
use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use proc_macro2_diagnostics::Level;
use quote::quote;
use rstml::Parser;
use rstml::ParserConfig;
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

// we must use `std::collections::HashSet` because thats what rstml uses
type HashSet<T> = std::collections::HashSet<T>;
/// definition for the rstml custom node, currently unused
pub(super) type RstmlCustomNode = rstml::Infallible;

/// Hashset of element tag names that should be self-closing.
#[derive(Debug, Clone, Resource)]
pub struct RstmlConfig {
	pub raw_text_elements: HashSet<&'static str>,
	pub self_closing_elements: HashSet<&'static str>,
}

impl Default for RstmlConfig {
	fn default() -> Self {
		Self {
			raw_text_elements: ["script", "style"].into_iter().collect(),
			self_closing_elements: [
				"area", "base", "br", "col", "embed", "hr", "img", "input",
				"link", "meta", "param", "source", "track", "wbr",
			]
			.into_iter()
			.collect(),
		}
	}
}

impl RstmlConfig {
	pub(super) fn into_parser(self) -> Parser<RstmlCustomNode> {
		let config = ParserConfig::new()
			.recover_block(true)
			.always_self_closed_elements(self.self_closing_elements)
			.raw_text_elements(self.raw_text_elements)
			// here we define the rsx! macro as the constant thats used
			// to resolve raw text blocks more correctly
			.macro_call_pattern(quote!(rsx! {%%}))
			.custom_node::<RstmlCustomNode>();
		Parser::new(config)
	}
}

/// A [`TokenStream`] representing [`rstml`] flavored rsx tokens.
#[derive(Debug, Clone, Deref, Component)]
#[require(MacroIdx)]
pub struct RstmlTokens(SendWrapper<TokenStream>);
impl RstmlTokens {
	pub fn new(tokens: TokenStream) -> Self { Self(SendWrapper::new(tokens)) }
	pub fn take(self) -> TokenStream { self.0.take() }
}

#[derive(Debug, Deref, DerefMut, Component)]
pub struct TokensDiagnostics(pub SendWrapper<Vec<Diagnostic>>);

impl TokensDiagnostics {
	pub fn new(value: Vec<Diagnostic>) -> Self { Self(SendWrapper::new(value)) }
	pub fn take(self) -> Vec<Diagnostic> { self.0.take() }
	pub fn into_tokens(self) -> Vec<TokenStream> {
		self.take()
			.into_iter()
			.map(|d| d.emit_as_expr_tokens())
			.collect()
	}
}


/// Replace the tokens with parsed [`RstmlNodes`], and apply a [`MacroIdx`]
pub(super) fn parse_rstml_tokens(
	_: TempNonSendMarker,
	mut commands: Commands,
	rstml_config: Res<RstmlConfig>,
	parser: NonSend<Parser<RstmlCustomNode>>,
	query: Populated<(Entity, &MacroIdx, &RstmlTokens), Added<RstmlTokens>>,
) -> Result {
	for (entity, macro_idx, handle) in query.iter() {
		let tokens = handle.clone().take();
		// this is the key to matching statically analyzed macros
		// with instantiated ones
		let (nodes, mut diagnostics) =
			parser.parse_recoverable(tokens).split_vec();

		let mut collected_elements = CollectedElements::default();

		let children = RstmlToWorld {
			file_path: &macro_idx.file,
			rstml_config: &rstml_config,
			collected_elements: &mut collected_elements,
			diagnostics: &mut diagnostics,
			commands: &mut commands,
			expr_idx: ExprIdxBuilder::new(),
		}
		.spawn_nodes(ParentContext::default(), nodes);
		commands
			.entity(entity)
			.remove::<RstmlTokens>()
			.insert((collected_elements, TokensDiagnostics::new(diagnostics)))
			.add_children(&children);
	}
	Ok(())
}

struct RstmlToWorld<'w, 's, 'a> {
	file_path: &'a WsPathBuf,
	rstml_config: &'a RstmlConfig,
	collected_elements: &'a mut CollectedElements,
	diagnostics: &'a mut Vec<Diagnostic>,
	commands: &'a mut Commands<'w, 's>,
	expr_idx: ExprIdxBuilder,
}

#[derive(Default, Copy, Clone, PartialEq, Eq)]
enum ParentContext {
	#[default]
	None,
	// attempt to mend RawText children
	StyleTag,
}


impl<'w, 's, 'a> RstmlToWorld<'w, 's, 'a> {
	/// Create an entity for each node in the vector.
	pub fn spawn_nodes(
		&mut self,
		parent_cx: ParentContext,
		nodes: Vec<Node<RstmlCustomNode>>,
	) -> Vec<Entity> {
		nodes
			.into_iter()
			.map(|node| {
				let entity = self.commands.spawn_empty().id();
				self.insert_node(parent_cx, entity, node);
				entity
			})
			.collect()
	}


	fn insert_node(
		&mut self,
		parent_cx: ParentContext,
		entity: Entity,
		node: Node<RstmlCustomNode>,
	) {
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
				let mut text = node.to_string_best();
				if parent_cx == ParentContext::StyleTag {
					text = self.mend_style_raw_text(&text);
				}
				self.commands.entity(entity).insert((
					TextNode(text),
					FileSpanOf::<TextNode>::new(file_span),
					SpanOf::<TextNode>::new(node_span),
				));
			}
			Node::Fragment(fragment) => {
				let children = self.spawn_nodes(parent_cx, fragment.children);
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

				let parent_cx = match tag_str.as_str() {
					"style" => ParentContext::StyleTag,
					_ => ParentContext::None,
				};
				let children = self.spawn_nodes(parent_cx, children);
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

				// once instantiated this block will be moved onto the parent
				// element, which will require an ExprIdx to resolve the block
				// this approach means only one block expr per element
				self.commands.entity(parent).insert(self.expr_idx.next());
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
	/// Best effort to mend styles parsed as token streams
	/// which introduces spaces around each character it considers
	/// to be a 'token', ie 'foo-bar' becomes 'foo - bar'
	fn mend_style_raw_text(&self, str: &str) -> String {
		{
			let replaced = str
				// em is not valid in rstml, we provide an alternative .em
				// hacks and attempts to fix back up the rstml parse
				.replace(".em", "em")
				.replace(". em", "em");
			// parsing rstml via token streams results in spaces around dashes.
			// Replace "A - Z" or "a - z" (with spaces) with "A-Z" or "a-z" using regex
			// Only if both sides are ASCII alphabetic
			let regex = regex::Regex::new(r"([A-Za-z]) - ([A-Za-z])").unwrap();
			regex.replace_all(&replaced, "$1-$2").to_string()
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
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;

	fn parse(tokens: TokenStream) -> (App, Entity) {
		let mut app = App::new();
		app.add_plugins(ParseRsxTokensPlugin);
		let entity = app.world_mut().spawn(RstmlTokens::new(tokens)).id();
		app.update();
		(app, entity)
	}


	#[test]
	fn works() {
		let (mut app, _) = parse(quote! {
			<span>
				<MyComponent client:load />
				<div/>
			</span>
		});

		expect(app.query_once::<&NodeTag>()).to_have_length(3);
		expect(app.query_once::<&ClientLoadDirective>()).to_have_length(1);
	}

	#[test]
	fn attribute_expr() {
		let (mut app, _) = parse(quote! {<div foo={7} bar="baz"/>});
		app.query_once::<&ExprIdx>().len().xpect().to_be(1);
	}

	#[test]
	fn style_tags() {
		let (mut app, _) = parse(quote! {
			<style>
			body{
				font-size: 1.em;
			}
			</style>
		});
		expect(app.query_once::<&LangContent>()[0]).to_be(
			&LangContent::InnerText("body { font-size : 1 em ; }".to_string()),
		);
	}
}

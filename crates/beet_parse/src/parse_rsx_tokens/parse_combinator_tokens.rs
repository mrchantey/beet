use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use beet_rsx_combinator::prelude::*;
use std::collections::HashSet;

/// A [`String`] of rsx tokens to be parsed into a node tree, which can then
/// be extracted into a [`Bundle`] [`TokenStream`] via [`tokenize_rsx`]
/// or [`tokenize_rsx_tokens`].
#[derive(Default, Component, Deref, Reflect)]
#[reflect(Default, Component)]
#[require(SnippetRoot)]
pub struct CombinatorTokens(String);

impl CombinatorTokens {
	/// Create a new [`CombinatorTokens`] from a string.
	pub fn new(tokens: impl Into<String>) -> Self { Self(tokens.into()) }
}


pub(super) fn parse_combinator_tokens(
	_: TempNonSendMarker,
	constants: Res<HtmlConstants>,
	mut commands: Commands,
	query: Populated<
		(Entity, &CombinatorTokens, &SnippetRoot),
		Added<CombinatorTokens>,
	>,
) -> bevy::prelude::Result {
	for (entity, tokens, snippet_root) in query.iter() {
		Builder {
			raw_text_elements: &constants.raw_text_elements,
			file_path: &snippet_root.file,
			commands: &mut commands,
			expr_idx: ExprIdxBuilder::new(),
		}
		.map_to_children(entity, tokens)?;
		commands.entity(entity).remove::<CombinatorTokens>();
	}
	Ok(())
}


/// For a given string of rsx, use [`beet_rsx_combinator`] to parse.
struct Builder<'w, 's, 'a> {
	// the content of these tags will not be parsed and instead inserted
	// as a [`TextNode`]
	raw_text_elements: &'a HashSet<&'static str>,
	file_path: &'a WsPathBuf,
	expr_idx: ExprIdxBuilder,
	commands: &'a mut Commands<'w, 's>,
}

impl<'w, 's, 'a> Builder<'w, 's, 'a> {
	fn map_to_children(
		mut self,
		root: Entity,
		rsx: &CombinatorTokens,
	) -> Result {
		let children = CombinatorParser::parse(&rsx).map_err(|e| {
			bevyhow!("Failed to parse Combinator RSX: {}", e.to_string())
		})?;

		let children = self.rsx_children("fragment", children)?;
		self.commands.entity(root).add_children(&children);
		Ok(())
	}

	// not ideal but we dont have spans for beet_rsx_combinator yet
	fn default_file_span(&self) -> FileSpan {
		FileSpan::new_for_file(&self.file_path)
	}
	/// insert a [`CombinatorExpr`] into the entity,
	/// as these are eventually collected into a [`NodeExpr`] each
	/// is assigned an [`ExprIdx`]
	fn rsx_parsed_expression(
		&mut self,
		entity: Entity,
		expr: RsxParsedExpression,
	) -> Result<()> {
		let partials = expr
			.inner()
			.into_iter()
			.map(|item| self.rsx_tokens_or_element(item))
			.collect::<Result<Vec<_>>>()?;

		let file_span = self.default_file_span();
		self.commands.entity(entity).insert((
			CombinatorExpr(partials),
			FileSpanOf::<CombinatorExpr>::new(file_span),
		));
		Ok(())
	}

	fn rsx_tokens_or_element(
		&mut self,
		tokens: RsxTokensOrElement,
	) -> Result<CombinatorExprPartial> {
		match tokens {
			RsxTokensOrElement::Tokens(tokens) => {
				CombinatorExprPartial::Tokens(tokens)
			}
			RsxTokensOrElement::Element(el) => {
				CombinatorExprPartial::Element(self.rsx_element(el)?)
			}
		}
		.xok()
	}

	fn rsx_fragment(&mut self, fragment: RsxFragment) -> Result<Entity> {
		let children = self.rsx_children("fragment", fragment.0)?;
		let file_span = self.default_file_span();
		self.commands
			.spawn((FragmentNode, FileSpanOf::<FragmentNode>::new(file_span)))
			.add_children(&children)
			.id()
			.xok()
	}

	fn rsx_element(&mut self, element: RsxElement) -> Result<Entity> {
		let (element_name, attributes, children, self_closing) = match element {
			RsxElement::Fragment(fragment) => {
				return self.rsx_fragment(fragment);
			}
			RsxElement::SelfClosing(el) => {
				(el.0, el.1, RsxChildren::default(), true)
			}
			RsxElement::Normal(el) => (el.0, el.1, el.2, false),
		};
		let tag_str = element_name.to_string();

		let file_span = self.default_file_span();

		let mut entity = self.commands.spawn((
			NodeTag(tag_str.clone()),
			FileSpanOf::<NodeTag>::new(self.default_file_span()),
		));

		if tag_str.starts_with(|c: char| c.is_uppercase()) {
			entity.insert((
				self.expr_idx.next(),
				TemplateNode,
				FileSpanOf::<TemplateNode>::new(file_span),
			));
		} else {
			entity.insert((
				ElementNode { self_closing },
				FileSpanOf::<ElementNode>::new(file_span),
			));
		}
		let entity = entity.id();

		attributes
			.0
			.into_iter()
			.map(|attr| self.spawn_attribute(entity, attr))
			.collect::<Result<Vec<_>>>()?;

		let children = self.rsx_children(&tag_str, children)?;
		self.commands.entity(entity).add_children(&children);

		entity.xok()
	}

	fn rsx_children(
		&mut self,
		tag_str: &str,
		children: RsxChildren,
	) -> Result<Vec<Entity>> {
		if self.raw_text_elements.contains(&tag_str) {
			vec![
				self.commands
					.spawn((
						TextNode::new(children.to_html()),
						FileSpanOf::<TextNode>::new(self.default_file_span()),
					))
					.id(),
			]
			.xok()
		} else {
			children
				.0
				.into_iter()
				.map(|child| self.rsx_child(child))
				.collect::<Result<Vec<_>>>()?
				.xok()
		}
	}

	fn rsx_child(&mut self, child: RsxChild) -> Result<Entity> {
		match child {
			RsxChild::Element(el) => self.rsx_element(el),
			RsxChild::Text(text) => self.rsx_text(text),
			RsxChild::CodeBlock(code_block) => {
				let entity =
					self.commands.spawn((BlockNode, self.expr_idx.next())).id();
				self.rsx_parsed_expression(entity, code_block)?;
				entity.xok()
			}
		}
	}

	fn rsx_text(&mut self, text: RsxText) -> Result<Entity> {
		self.commands
			.spawn((
				TextNode::new(text.0.to_string()),
				FileSpanOf::<TextNode>::new(self.default_file_span()),
			))
			.id()
			.xok()
	}

	fn spawn_attribute(
		&mut self,
		parent: Entity,
		attribute: RsxAttribute,
	) -> Result<()> {
		match attribute {
			RsxAttribute::Spread(value) => {
				let entity = self
					.commands
					.spawn((
						AttributeOf::new(parent),
						FileSpanOf::<NodeExpr>::new(self.default_file_span()),
					))
					.id();
				self.commands.entity(parent).insert(self.expr_idx.next());
				self.rsx_parsed_expression(entity, value)?;
			}
			RsxAttribute::Named(name, value) => {
				let mut entity = self.commands.spawn((
					AttributeKey::new(name.to_string()),
					AttributeOf::new(parent),
					FileSpanOf::<AttributeOf>::new(self.default_file_span()),
				));
				match value {
					RsxAttributeValue::Default => {
						// key only attribute
					}
					RsxAttributeValue::Boolean(val) => {
						let val = val.0;
						entity.insert((
							NodeExpr::new(syn::parse_quote! {#val}),
							val.into_bundle(),
						));
					}
					RsxAttributeValue::Number(val) => {
						let val = val.0;
						entity.insert((
							NodeExpr::new(syn::parse_quote! {#val}),
							val.into_bundle(),
						));
					}
					RsxAttributeValue::Str(val) => {
						let val = val.to_string_unquoted();
						entity.insert((
							NodeExpr::new(syn::parse_quote! {#val}),
							TextNode::new(val),
						));
					}
					RsxAttributeValue::Element(value) => {
						let id = entity.id();
						// get ExprIdx before evaluating element
						let expr_id = self.expr_idx.next();
						let child = self.rsx_element(value)?;
						self.commands.entity(id).insert((
							expr_id,
							CombinatorExpr(vec![
								CombinatorExprPartial::Element(child),
							]),
						));
					}
					RsxAttributeValue::CodeBlock(value) => {
						entity.insert(self.expr_idx.next());
						let entity = entity.id();
						self.rsx_parsed_expression(entity, value)?;
					}
				}
			}
		}
		.xok()
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use proc_macro2::TokenStream;

	fn parse(str: &str) -> TokenStream {
		ParseRsxTokens::combinator_to_rsx(str, WsPathBuf::new(file!())).unwrap()
	}

	#[test]
	fn element() { "<br/>".xmap(parse).xpect_snapshot(); }
	#[test]
	fn fragment() {
		"<br/><br/>"
			.xmap(|str| {
				ParseRsxTokens::combinator_to_rsx(str, WsPathBuf::new(file!()))
					.unwrap()
			})
			.xpect_snapshot();
	}
	#[test]
	fn unclosed() { "<div align=\"center\" />".xmap(parse).xpect_snapshot(); }

	#[test]
	fn text() { "<div>hello</div>".xmap(parse).xpect_snapshot(); }

	#[test]
	fn block() { r#"{"hello"}"#.xmap(parse).xpect_snapshot(); }



	#[test]
	fn element_attributes_default() {
		"<br foo />".xmap(parse).xpect_snapshot();
	}

	#[test]
	fn element_attributes_string() {
		"<br foo=\"bar\"/>".xmap(parse).xpect_snapshot();
	}

	#[test]
	fn element_attributes_bool() {
		"<br foo=true />".xmap(parse).xpect_snapshot();
	}

	#[test]
	fn element_attributes_number() {
		"<br foo=20 />".xmap(parse).xpect_snapshot();
	}

	#[test]
	fn element_attributes_block_value() {
		"<br foo={bar} />".xmap(parse).xpect_snapshot();
	}
	#[test]
	fn element_attributes_spread() {
		"<br {...bar} />".xmap(parse).xpect_snapshot();
	}

	#[test]
	fn element_attributes_element() {
		"<br foo={<br/>} />".xmap(parse).xpect_snapshot();
	}

	#[test]
	fn element_attributes_mixed() {
		"<br foo={
			let bar = <br/>;
			bar
		} />"
			.xmap(parse)
			.xpect_snapshot();
	}

	#[test]
	fn template_attributes_default() {
		"<MyTemplate foo />".xmap(parse).xpect_snapshot();
	}

	#[test]
	fn template_attributes_string() {
		"<MyTemplate foo=\"bar\"/>".xmap(parse).xpect_snapshot();
	}

	#[test]
	fn template_attributes_bool() {
		"<MyTemplate foo=true />".xmap(parse).xpect_snapshot();
	}

	#[test]
	fn template_attributes_number() {
		"<MyTemplate foo=20 />".xmap(parse).xpect_snapshot();
	}

	#[test]
	fn template_attributes_ident() {
		"<MyTemplate foo={bar} />".xmap(parse).xpect_snapshot();
	}

	#[test]
	fn template_attributes_element() {
		"<MyTemplate foo={<br/>} />".xmap(parse).xpect_snapshot();
	}

	#[test]
	fn template_attributes_mixed() {
		r#"<MyTemplate foo={
			let bar = <br/>;
			bar
		} />"#
			.xmap(parse)
			.xpect_snapshot();
	}
	#[cfg(feature = "css")]
	#[test]
	fn style() {
		r#"
<div> hello world </div>
<style>
	main{
		padding-top: 2em;
		display: flex;
		flex-direction: column;
		align-items: center;
		height: 100vh;
	}
	a {
		color: #90ee90;
	}
	a:visited {
		color: #3399ff;
	}
</style>
<style scope:global>
	body{
		font-size: 1.4em;
		font-family: system-ui, sans-serif;
		background: black;
		color: white;
	}
</style>
"#
		.xmap(parse)
		.xpect_snapshot();
	}
	#[test]
	#[ignore = "todo combinator raw text"]
	fn preserves_whitespace() {
		let out = ParseRsxTokens::combinator_to_rsx(
			r#"
<pre><code class="language-rust">// A simple Rust function
fn fibonacci(n: u32) -&gt; u32 {
    match n {
        0 =&gt; 0,
        1 =&gt; 1,
        _ =&gt; fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() {
    let result = fibonacci(10);
    println!("The 10th Fibonacci number is: {}", result);
}
</code></pre>
		"#,
			WsPathBuf::new(file!()),
		)
		.unwrap();
		out.to_string().xpect_contains("\nfn main()");
		out.xpect_snapshot();
	}
}

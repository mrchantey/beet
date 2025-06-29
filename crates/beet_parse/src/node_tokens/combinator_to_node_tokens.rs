use crate::prelude::*;
use anyhow::Result;
use beet_common::prelude::*;
use beet_rsx_combinator::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;

/// A [`String`] of rsx tokens to be parsed into a node tree, which can then
/// be extracted into a [`Bundle`] [`TokenStream`] via [`tokenize_bundle`]
/// or [`tokenize_bundle_tokens`].
#[derive(Default, Component, Deref, Reflect)]
#[reflect(Default, Component)]
#[require(MacroIdx)]
pub struct CombinatorTokens(String);

impl CombinatorTokens {
	/// Create a new [`CombinatorTokens`] from a string.
	pub fn new(tokens: impl Into<String>) -> Self { Self(tokens.into()) }
}

pub fn combinator_to_node_tokens_plugin(app: &mut App) {
	app.add_systems(Update, combinator_to_node_tokens.in_set(ImportNodesStep));
}


fn combinator_to_node_tokens(
	_: TempNonSendMarker,
	mut commands: Commands,
	query: Populated<
		(Entity, &CombinatorTokens, &MacroIdx),
		Added<CombinatorTokens>,
	>,
) -> bevy::prelude::Result {
	for (entity, tokens, macro_idx) in query.iter() {
		Builder {
			verbatim_tags: &["script", "style", "code"],
			file_path: &macro_idx.file,
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
	verbatim_tags: &'a [&'a str],
	file_path: &'a WsPathBuf,
	expr_idx: ExprIdxBuilder,
	commands: &'a mut Commands<'w, 's>,
}

impl<'w, 's, 'a> Builder<'w, 's, 'a> {
	fn map_to_children(
		mut self,
		root: Entity,
		rsx: &CombinatorTokens,
	) -> Result<()> {

		let (expr, remaining) = parse(&rsx).map_err(|e| {
			anyhow::anyhow!("Failed to parse Combinator RSX: {}", e.to_string())
		})?;
		if !remaining.is_empty() {
			return Err(anyhow::anyhow!(
				"Unparsed input remaining: {}",
				remaining
			));
		}

		// unlike rstml, we dont know what the root [`RsxParsedExpression`] will
		// evaluate to, so just spawn as a single child entity.
		let expr_root = self.commands.spawn_empty().id();

		self.rsx_parsed_expression(expr_root, expr)?;
		self.commands.entity(root).add_child(expr_root);
		Ok(())
	}

	// not ideal but we dont have spans for beet_rsx_combinator yet
	fn default_file_span(&self) -> FileSpan {
		FileSpan::new_for_file(&self.file_path)
	}
	/// insert a [`CombinatorExpr`] into the entity
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
				TemplateNode,
				self.expr_idx.next(),
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
		if self.verbatim_tags.contains(&tag_str) {
			vec![
				self.commands
					.spawn((
						TextNode(children.to_html()),
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
				let entity = self.commands.spawn_empty().id();
				self.rsx_parsed_expression(entity, code_block)?;
				entity.xok()
			}
		}
	}

	fn rsx_text(&mut self, text: RsxText) -> Result<Entity> {
		self.commands
			.spawn((
				TextNode(text.0.to_string()),
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
				self.rsx_parsed_expression(entity, value)?;
			}
			RsxAttribute::Named(name, value) => {
				let mut entity = self.commands.spawn((
					AttributeKey::new(name.to_string()),
					AttributeOf::new(parent),
					FileSpanOf::<AttributeOf>::new(self.default_file_span()),
				));
				match value {
					RsxAttributeValue::Default => {}
					RsxAttributeValue::Boolean(val) => {
						let val = val.0;
						entity.insert((
							NodeExpr::new(syn::parse_quote! {#val}),
							AttributeLit::new(val),
						));
					}
					RsxAttributeValue::Number(val) => {
						let val = val.0;
						entity.insert((
							NodeExpr::new(syn::parse_quote! {#val}),
							AttributeLit::new(val),
						));
					}
					RsxAttributeValue::Str(val) => {
						let val = val.to_string_unquoted();
						entity.insert((
							NodeExpr::new(syn::parse_quote! {#val}),
							AttributeLit::new(val),
						));
					}
					RsxAttributeValue::Element(value) => {
						let id = entity.id();
						let child = self.rsx_element(value)?;
						self.commands.entity(id).insert(CombinatorExpr(vec![
							CombinatorExprPartial::Element(child),
						]));
					}
					RsxAttributeValue::CodeBlock(value) => {
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
	use beet_utils::prelude::*;
	use quote::quote;
	use sweet::prelude::*;

	fn parse(str: &str) -> Matcher<String> {
		tokenize_combinator(str, WsPathBuf::new(file!()))
			.unwrap()
			.to_string()
			.xpect()
	}

	#[test]
	fn element() {
		"<br/>".xmap(parse).to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
				{(
					NodeTag(String::from("br")),
					ElementNode{self_closing:true}
				)}.into_node_bundle())]}
			)}
			.to_string(),
		);
	}
	#[test]
	fn fragment() {
		"<br/><br/>"
			.xmap(|str| {
				tokenize_combinator(str, WsPathBuf::new(file!()))
					.unwrap()
					.to_string()
					.xpect()
			})
			.to_be_str(
				quote! {(
					BeetRoot,
					InstanceRoot,
					MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
					FragmentNode,
					related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
					{(
						NodeTag(String::from("br")),
						ElementNode{self_closing:true}
					)(
						NodeTag(String::from("br")),
						ElementNode{self_closing:true}
					)}.into_node_bundle())]}
				)}
				.to_string(),
			);
	}
	#[test]
	fn unclosed() {
		"<div align=\"center\" />".xmap(parse).to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
				{(
					NodeTag(String::from("div")),
					ElementNode{self_closing:true},
					related!(Attributes[(
						AttributeKey::new("align"),
						OnSpawnTemplate::new_insert("center".into_attribute_bundle())
					)])
				)}.into_node_bundle())]}
			)}
			.to_string(),
		);
	}

	#[test]
	fn text() {
		"<div>hello</div>".xmap(parse).to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
				{(
					NodeTag(String::from("div")),
					ElementNode{self_closing:false},
					related!{Children[TextNode(String::from("hello"))]}
				)}.into_node_bundle())]}
			)}
			.to_string(),
		);
	}
	#[test]
	fn element_attributes_default() {
		"<br foo />".xmap(parse).to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
				{(
					NodeTag(String::from("br")),
					ElementNode{self_closing:true},
					related!(Attributes[AttributeKey::new("foo")])
				)}.into_node_bundle())]}
			)}
			.to_string(),
		);
	}

	#[test]
	fn element_attributes_string() {
		"<br foo=\"bar\"/>".xmap(parse).to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
				{(
					NodeTag(String::from("br")),
					ElementNode{self_closing:true},
					related!(Attributes[(
						AttributeKey::new("foo"),
						OnSpawnTemplate::new_insert("bar".into_attribute_bundle())
					)])
				)}.into_node_bundle())]}
			)}
			.to_string(),
		);
	}

	#[test]
	fn element_attributes_bool() {
		"<br foo=true />".xmap(parse).to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
				{(
					NodeTag(String::from("br")),
					ElementNode{self_closing:true},
					related!(Attributes[(
						AttributeKey::new("foo"),
						OnSpawnTemplate::new_insert(true.into_attribute_bundle())
					)])
				)}.into_node_bundle())]}
			)}
			.to_string(),
		);
	}

	#[test]
	fn element_attributes_number() {
		"<br foo=20 />".xmap(parse).to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
				{(
					NodeTag(String::from("br")),
					ElementNode{self_closing:true},
					related!(Attributes[(
						AttributeKey::new("foo"),
						OnSpawnTemplate::new_insert(20f64.into_attribute_bundle())
					)])
				)}.into_node_bundle())]}
			)}
			.to_string(),
		);
	}

	#[test]
	fn element_attributes_ident() {
		"<br foo={bar} />".xmap(parse).to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
				{(
					NodeTag(String::from("br")),
					ElementNode{self_closing:true},
					related!(Attributes[(
						AttributeKey::new("foo"),
						OnSpawnTemplate::new_insert(#[allow(unused_braces)]{ bar }.into_attribute_bundle())
					)])
				)}.into_node_bundle())]}
			)}
			.to_string(),
		);
	}

	#[test]
	fn element_attributes_element() {
		"<br foo={<br/>} />".xmap(parse).to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
				{(
					NodeTag(String::from("br")),
					ElementNode{self_closing:true},
					related!(Attributes[(
						AttributeKey::new("foo"),
							OnSpawnTemplate::new_insert(#[allow(unused_braces)]{(
								NodeTag(String::from("br")),
								ElementNode { self_closing: true }
							)}.into_attribute_bundle())
					)])
				)}.into_node_bundle())]}
			)}
			.to_string(),
		);
	}

	#[test]
	fn element_attributes_mixed() {
		"<br foo={
			let bar = <br/>;
			bar
		} />"
			.xmap(parse)
			.to_be_str(
				quote! {(
					BeetRoot,
					InstanceRoot,
					MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
					FragmentNode,
					related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
					{(
						NodeTag(String::from("br")),
						ElementNode{self_closing:true},
						related!(Attributes[(
							AttributeKey::new("foo"),
							OnSpawnTemplate::new_insert(#[allow(unused_braces)]{
								let bar = (
									NodeTag(String::from("br")),
									ElementNode{self_closing:true}
								);
								bar
							}.into_attribute_bundle())
						)])
					)}.into_node_bundle())]}
				)}
				.to_string(),
			);
	}

	#[test]
	fn template_attributes_default() {
		"<MyTemplate foo />".xmap(parse).to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
				{(
					ExprIdx(0u32),
					NodeTag(String::from("MyTemplate")),
					FragmentNode,
					TemplateNode,
					OnSpawnTemplate::new_insert(#[allow(unused_braces)]{
						let template = <MyTemplate as Props>::Builder::default().foo(true).build();
						TemplateRoot::spawn(Spawn(template.into_node_bundle()))
					}.into_node_bundle())
				)}.into_node_bundle())]}
			)}
			.to_string(),
		);
	}

	#[test]
	fn template_attributes_string() {
		"<MyTemplate foo=\"bar\"/>".xmap(parse).to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
				{(
					ExprIdx(0u32),
					NodeTag(String::from("MyTemplate")),
					FragmentNode,
					TemplateNode,
					OnSpawnTemplate::new_insert(#[allow(unused_braces)]{
						let template = <MyTemplate as Props>::Builder::default().foo("bar").build();
						TemplateRoot::spawn(Spawn(template.into_node_bundle()))
					}.into_node_bundle())
				)}.into_node_bundle())]}
			)}
			.to_string(),
		);
	}

	#[test]
	fn template_attributes_bool() {
		"<MyTemplate foo=true />".xmap(parse).to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
				{(
					ExprIdx(0u32),
					NodeTag(String::from("MyTemplate")),
					FragmentNode,
					TemplateNode,
					OnSpawnTemplate::new_insert(#[allow(unused_braces)]{
						let template = <MyTemplate as Props>::Builder::default().foo(true).build();
						TemplateRoot::spawn(Spawn(template.into_node_bundle()))
					}.into_node_bundle())
				)}.into_node_bundle())]}
			)}
			.to_string(),
		);
	}

	#[test]
	fn template_attributes_number() {
		"<MyTemplate foo=20 />".xmap(parse).to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
				{(
					ExprIdx(0u32),
					NodeTag(String::from("MyTemplate")),
					FragmentNode,
					TemplateNode,
					OnSpawnTemplate::new_insert(#[allow(unused_braces)]{
						let template = <MyTemplate as Props>::Builder::default().foo(20f64).build();
						TemplateRoot::spawn(Spawn(template.into_node_bundle()))
					}.into_node_bundle())
				)}.into_node_bundle())]}
			)}
			.to_string(),
		);
	}

	#[test]
	fn template_attributes_ident() {
		"<MyTemplate foo={bar} />".xmap(parse).to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
				{(
					ExprIdx(0u32),
					NodeTag(String::from("MyTemplate")),
					FragmentNode,
					TemplateNode,
					OnSpawnTemplate::new_insert(#[allow(unused_braces)]{
						let template = <MyTemplate as Props>::Builder::default().foo(#[allow(unused_braces)]{ bar }).build();
						TemplateRoot::spawn(Spawn(template.into_node_bundle()))
					}.into_node_bundle())
				)}.into_node_bundle())]}
			)}
			.to_string(),
		);
	}

	#[test]
	fn template_attributes_element() {
		"<MyTemplate foo={<br/>} />".xmap(parse).to_be_str(
			quote! {(
				BeetRoot,
				InstanceRoot,
				MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
				FragmentNode,
				related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
				{(
					ExprIdx(0u32),
					NodeTag(String::from("MyTemplate")),
					FragmentNode,
					TemplateNode,
					OnSpawnTemplate::new_insert(#[allow(unused_braces)]{
						let template = <MyTemplate as Props>::Builder::default().foo(#[allow(unused_braces)]{ (
							NodeTag(String::from("br")),
							ElementNode { self_closing: true }
						) }).build();
						TemplateRoot::spawn(Spawn(template.into_node_bundle()))
					}.into_node_bundle())
				)}.into_node_bundle())]}
			)}
			.to_string(),
		);
	}

	#[test]
	fn template_attributes_mixed() {
		"<MyTemplate foo={
			let bar = <br/>;
			bar
		} />"
			.xmap(parse)
			.to_be_str(
				quote! {(
					BeetRoot,
					InstanceRoot,
					MacroIdx{file:WsPathBuf::new("crates/beet_parse/src/node_tokens/combinator_to_node_tokens.rs"),start:LineCol{line:1u32,col:0u32}},
					FragmentNode,
					related!{Children[OnSpawnTemplate::new_insert(#[allow(unused_braces)]
					{(
						ExprIdx(0u32),
						NodeTag(String::from("MyTemplate")),
						FragmentNode,
						TemplateNode,
						OnSpawnTemplate::new_insert(#[allow(unused_braces)]{
							let template = <MyTemplate as Props>::Builder::default().foo(#[allow(unused_braces)]{
								let bar = (
									NodeTag(String::from("br")),
									ElementNode{self_closing:true}
								);
								bar
							}).build();
						TemplateRoot::spawn(Spawn(template.into_node_bundle()))
					}.into_node_bundle())
				)}.into_node_bundle())]}
				)}
				.to_string(),
			);
	}
}

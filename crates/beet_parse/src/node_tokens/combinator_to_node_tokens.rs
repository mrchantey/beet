use crate::prelude::*;
use anyhow::Result;
use beet_common::prelude::*;
use beet_rsx_combinator::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use sweet::prelude::*;

pub fn combinator_to_bundle(
	tokens: &str,
	source_file: WorkspacePathBuf,
) -> Result<TokenStream> {
	TokensApp::with(|app| {
		let entity = app
			.world_mut()
			.spawn((
				SourceFile::new(source_file),
				NodeTokensToBundle::default().exclude_errors(),
				CombinatorToNodeTokens(tokens.to_string()),
			))
			.id();
		app.update();
		let result = app
			.world_mut()
			.entity_mut(entity)
			.take::<BundleTokens>()
			.map(|tokens| tokens.take())
			.ok_or_else(|| {
				anyhow::anyhow!("Internal Error: Expected token stream")
			})?
			.xok();
		app.world_mut().entity_mut(entity).despawn();
		result
	})
}


/// Provide a string of rsx tokens to be parsed into a node tree.
#[derive(Default, Component, Deref, Reflect)]
#[reflect(Default, Component)]
pub struct CombinatorToNodeTokens(pub String);


pub fn combinator_to_node_tokens_plugin(app: &mut App) {
	app.add_systems(Update, combinator_to_node_tokens.in_set(ImportNodesStep));
}


fn combinator_to_node_tokens(
	mut commands: Commands,
	query: Populated<(Entity, &CombinatorToNodeTokens, Option<&SourceFile>)>,
) -> bevy::prelude::Result {
	for (entity, tokens, source_file) in query.iter() {
		let default_source_file = WorkspacePathBuf::default();
		Builder {
			verbatim_tags: &["script", "style", "code"],
			source_file: source_file.map_or(&default_source_file, |sf| &sf),
			rusty_tracker: RustyTrackerBuilder::default(),
			commands: &mut commands,
		}
		.map_to_children(entity, tokens)?;
		commands.entity(entity).remove::<CombinatorToNodeTokens>();
	}
	Ok(())
}


/// For a given string of rsx, use [`beet_rsx_combinator`] to parse.
struct Builder<'w, 's, 'a> {
	verbatim_tags: &'a [&'a str],
	source_file: &'a WorkspacePathBuf,
	rusty_tracker: RustyTrackerBuilder,
	commands: &'a mut Commands<'w, 's>,
}

impl<'w, 's, 'a> Builder<'w, 's, 'a> {
	fn map_to_children(
		mut self,
		root: Entity,
		rsx: &CombinatorToNodeTokens,
	) -> Result<()> {
		let (expr, remaining) = parse(&rsx).map_err(|e| {
			anyhow::anyhow!("Failed to parse HTML: {}", e.to_string())
		})?;
		if !remaining.is_empty() {
			return Err(anyhow::anyhow!(
				"Unparsed input remaining: {}",
				remaining
			));
		}
		// add as a child to keep consistency with rstml_to_tokens
		let child = self.commands.spawn_empty().id();
		self.rsx_parsed_expression(child, expr)?;
		self.commands
			.entity(root)
			.insert(ItemOf::<(), _>::new(FileSpan::new_for_file(
				&self.source_file,
			)))
			.add_child(child);
		Ok(())
	}

	// not ideal but we dont have spans for beet_rsx_combinator yet
	fn default_file_span(&self) -> FileSpan {
		FileSpan::new_for_file(&self.source_file)
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
			ItemOf::<CombinatorExpr, _>::new(file_span),
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

	fn rsx_element(&mut self, element: RsxElement) -> Result<Entity> {
		let (element_name, attributes, children, self_closing) = match element {
			RsxElement::SelfClosing(el) => {
				(el.0, el.1, RsxChildren::default(), true)
			}
			RsxElement::Normal(el) => (el.0, el.1, el.2, false),
		};
		let tag_str = element_name.to_string();

		let children = self.rsx_children(&tag_str, children)?;
		let file_span = self.default_file_span();

		let mut entity = self.commands.spawn((
			NodeTag(tag_str.clone()),
			ItemOf::<NodeTag, _>::new(self.default_file_span()),
		));

		entity.add_children(&children);


		if tag_str.starts_with(|c: char| c.is_uppercase()) {
			// yes we get the tracker after its children, its fine as long
			// as its consistent with other parsers.
			let tracker =
				self.rusty_tracker.next_rsx_el(&element_name, &attributes);
			entity.insert((
				TemplateNode,
				ItemOf::<TemplateNode, _>::new(tracker),
				ItemOf::<TemplateNode, _>::new(file_span),
			));
		} else {
			entity.insert((
				ElementNode { self_closing },
				ItemOf::<TemplateNode, _>::new(file_span),
			));
		}
		let entity = entity.id();
		attributes
			.0
			.into_iter()
			.map(|attr| self.spawn_attribute(entity, attr))
			.collect::<Result<Vec<_>>>()?;

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
						ItemOf::<TextNode, _>::new(self.default_file_span()),
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
				ItemOf::<TextNode, _>::new(self.default_file_span()),
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
						ItemOf::<AttributeExpr, _>::new(
							self.default_file_span(),
						),
					))
					.id();
				self.rsx_parsed_expression(entity, value)?;
			}
			RsxAttribute::Named(name, value) => {
				let key = name.to_string();

				let mut entity = self.commands.spawn((
					AttributeOf::new(parent),
					AttributeKeyExpr::new(syn::parse_quote!(#key)),
					ItemOf::<AttributeOf, _>::new(self.default_file_span()),
				));

				match value {
					RsxAttributeValue::Default => {
						entity.insert(AttributeLit::new(key, None));
					}
					RsxAttributeValue::Boolean(val) => {
						let val = val.0;
						entity.insert((
							AttributeLit::new(key, Some(val.to_string())),
							AttributeValueExpr::new(syn::parse_quote! {#val}),
						));
					}
					RsxAttributeValue::Number(val) => {
						let val = val.0;
						entity.insert((
							AttributeLit::new(key, Some(val.to_string())),
							AttributeValueExpr::new(syn::parse_quote! {#val}),
						));
					}
					RsxAttributeValue::Str(val) => {
						let val = val.to_string_unquoted();
						entity.insert((
							AttributeLit::new(key, Some(val.to_string())),
							AttributeValueExpr::new(syn::parse_quote! {#val}),
						));
					}
					RsxAttributeValue::Element(value) => {
						let id = entity.id();
						let child = self.rsx_element(value)?;
						self.commands.entity(id).insert((
							CombinatorExpr(vec![
								CombinatorExprPartial::Element(child),
							]),
							AttributeLit::new(key, None),
						));
					}
					RsxAttributeValue::CodeBlock(value) => {
						entity.insert(AttributeLit::new(key, None));
						let id = entity.id();
						self.rsx_parsed_expression(id, value)?;
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
	use quote::quote;
	use sweet::prelude::*;

	fn parse(str: &str) -> Matcher<String> {
		combinator_to_bundle(str, WorkspacePathBuf::new(file!()))
			.unwrap()
			.to_string()
			.xpect()
	}

	#[test]
	fn element() {
		"<br/>".xmap(parse).to_be(
			quote! {{(
				NodeTag(String::from("br")),
				ElementNode{self_closing:true}
			)}}
			.to_string(),
		);
	}
	#[test]
	fn unclosed() {
		"<div align=\"center\" />".xmap(parse).to_be(
			quote! {{(
				NodeTag(String::from("div")),
				ElementNode{self_closing:true},
				related!(Attributes[(
					"align".into_attr_key_bundle(),
					"center".into_attr_val_bundle()
				)])
			)}}
			.to_string(),
		);
	}

	#[test]
	fn text() {
		"<div>hello</div>".xmap(parse).to_be(
			quote! {{(
				NodeTag(String::from("div")),
				ElementNode{self_closing:false},
				children![TextNode(String::from("hello"))]
			)}}
			.to_string(),
		);
	}
	#[test]
	fn element_attributes() {
		// default
		"<br foo />".xmap(parse).to_be(
			quote! {{(
				NodeTag(String::from("br")),
				ElementNode{self_closing:true},
				related!(Attributes["foo".into_attr_key_bundle()])
			)}}
			.to_string(),
		);
		// string
		"<br foo=\"bar\"/>".xmap(parse).to_be(
			quote! {{(
				NodeTag(String::from("br")),
				ElementNode{self_closing:true},
				related!(Attributes[(
					"foo".into_attr_key_bundle(),
					"bar".into_attr_val_bundle()
				)])
			)}}
			.to_string(),
		);
		// bool
		"<br foo=true />".xmap(parse).to_be(
			quote! {{(
				NodeTag(String::from("br")),
				ElementNode{self_closing:true},
				related!(Attributes[(
					"foo".into_attr_key_bundle(),
					true.into_attr_val_bundle()
				)])
			)}}
			.to_string(),
		);
		// number
		"<br foo=20 />".xmap(parse).to_be(
			quote! {{(
				NodeTag(String::from("br")),
				ElementNode{self_closing:true},
				related!(Attributes[(
					"foo".into_attr_key_bundle(),
					20f64.into_attr_val_bundle()
				)])
			)}}
			.to_string(),
		);
		// ident
		"<br foo={bar} />".xmap(parse).to_be(
			quote! {{(
				NodeTag(String::from("br")),
				ElementNode{self_closing:true},
				related!(Attributes[(
					"foo".into_attr_key_bundle(),
					{ bar }.into_attr_val_bundle()
				)])
			)}}
			.to_string(),
		);
		// element
		"<br foo={<br/>} />".xmap(parse).to_be(
			quote! {{(
				NodeTag(String::from("br")),
				ElementNode{self_closing:true},
				related!(Attributes[(
					"foo".into_attr_key_bundle(),
						{(
							NodeTag(String::from("br")),
							ElementNode { self_closing: true }
						)}.into_attr_val_bundle()
				)])
			)}}
			.to_string(),
		);
		// mixed
		"<br foo={
			let bar = <br/>;
			bar
		} />"
			.xmap(parse)
			.to_be(
				quote! {{(
					NodeTag(String::from("br")),
					ElementNode{self_closing:true},
					related!(Attributes[(
						"foo".into_attr_key_bundle(),
						{
							let bar = (
								NodeTag(String::from("br")),
								ElementNode{self_closing:true}
							);
							bar
						}.into_attr_val_bundle()
					)])
				)}}
				.to_string(),
			);
	}
	#[test]
	fn template_attributes() {
		// default
		"<MyTemplate foo />".xmap(parse).to_be(
			quote! {{(
				NodeTag(String::from("MyTemplate")),
				FragmentNode,
				ItemOf::<beet_common::node::rsx_nodes::TemplateNode, beet_common::templating::rusty_tracker::RustyTracker>{
					value: RustyTracker{index: 0u32, tokens_hash: 10188144591803042436u64},
					phantom: std::marker::PhantomData::<beet_common::node::rsx_nodes::TemplateNode>
				},
				{
					let template = <MyTemplate as Props>::Builder::default().foo(true).build();
					#[allow(unused_braces)]
					(TemplateRoot::spawn(Spawn(template.into_node_bundle())))
				}
			)}}
			.to_string(),
		);
		// string
		"<MyTemplate foo=\"bar\"/>".xmap(parse).to_be(
			quote! {{(
				NodeTag(String::from("MyTemplate")),
				FragmentNode,
				ItemOf::<beet_common::node::rsx_nodes::TemplateNode, beet_common::templating::rusty_tracker::RustyTracker>{
					value: RustyTracker{index: 0u32, tokens_hash: 4889923030152902413u64},
					phantom: std::marker::PhantomData::<beet_common::node::rsx_nodes::TemplateNode>
				},
				{
					let template = <MyTemplate as Props>::Builder::default().foo("bar").build();
					#[allow(unused_braces)]
					(TemplateRoot::spawn(Spawn(template.into_node_bundle())))
				}
			)}}
			.to_string(),
		);
		// bool
		"<MyTemplate foo=true />".xmap(parse).to_be(
			quote! {{(
				NodeTag(String::from("MyTemplate")),
				FragmentNode,
				ItemOf::<beet_common::node::rsx_nodes::TemplateNode, beet_common::templating::rusty_tracker::RustyTracker>{
					value: RustyTracker{index: 0u32, tokens_hash: 15310342799507411129u64},
					phantom: std::marker::PhantomData::<beet_common::node::rsx_nodes::TemplateNode>
				},
				{
					let template = <MyTemplate as Props>::Builder::default().foo(true).build();
					#[allow(unused_braces)]
					(TemplateRoot::spawn(Spawn(template.into_node_bundle())))
				}
			)}}
			.to_string(),
		);
		// number
		"<MyTemplate foo=20 />".xmap(parse).to_be(
			quote! {{(
				NodeTag(String::from("MyTemplate")),
				FragmentNode,
				ItemOf::<beet_common::node::rsx_nodes::TemplateNode, beet_common::templating::rusty_tracker::RustyTracker>{
					value: RustyTracker{index: 0u32, tokens_hash: 11502427431261689614u64},
					phantom: std::marker::PhantomData::<beet_common::node::rsx_nodes::TemplateNode>
				},
				{
					let template = <MyTemplate as Props>::Builder::default().foo(20f64).build();
					#[allow(unused_braces)]
					(TemplateRoot::spawn(Spawn(template.into_node_bundle())))
				}
			)}}
			.to_string(),
		);
		// ident
		"<MyTemplate foo={bar} />".xmap(parse).to_be(
			quote! {{(
				NodeTag(String::from("MyTemplate")),
				FragmentNode,
				ItemOf::<beet_common::node::rsx_nodes::TemplateNode, beet_common::templating::rusty_tracker::RustyTracker>{
					value: RustyTracker{index: 0u32, tokens_hash: 9730180295528883542u64},
					phantom: std::marker::PhantomData::<beet_common::node::rsx_nodes::TemplateNode>
				},
				{
					let template = <MyTemplate as Props>::Builder::default().foo({ bar }).build();
					#[allow(unused_braces)]
					(TemplateRoot::spawn(Spawn(template.into_node_bundle())))
				}
			)}}
			.to_string(),
		);
		// element
		"<MyTemplate foo={<br/>} />".xmap(parse).to_be(
			quote! {{(
				NodeTag(String::from("MyTemplate")),
				FragmentNode,
				ItemOf::<beet_common::node::rsx_nodes::TemplateNode, beet_common::templating::rusty_tracker::RustyTracker>{
					value: RustyTracker{index: 0u32, tokens_hash: 7310951454258190932u64},
					phantom: std::marker::PhantomData::<beet_common::node::rsx_nodes::TemplateNode>
				},
				{
					let template = <MyTemplate as Props>::Builder::default().foo({ (
						NodeTag(String::from("br")),
						ElementNode { self_closing: true }
					) }).build();
					#[allow(unused_braces)]
					(TemplateRoot::spawn(Spawn(template.into_node_bundle())))
				}
			)}}
			.to_string(),
		);
		// mixed
		"<MyTemplate foo={
			let bar = <br/>;
			bar
		} />"
			.xmap(parse)
			.to_be(
				quote! {{(
					NodeTag(String::from("MyTemplate")),
					FragmentNode,
					ItemOf::<beet_common::node::rsx_nodes::TemplateNode, beet_common::templating::rusty_tracker::RustyTracker>{
						value: RustyTracker{index: 0u32, tokens_hash: 577660515964029912u64},
						phantom: std::marker::PhantomData::<beet_common::node::rsx_nodes::TemplateNode>
					},
					{
						let template = <MyTemplate as Props>::Builder::default().foo({
							let bar = (
								NodeTag(String::from("br")),
								ElementNode{self_closing:true}
							);
							bar
						}).build();
						#[allow(unused_braces)]
						(TemplateRoot::spawn(Spawn(template.into_node_bundle())))
					}
				)}}
				.to_string(),
			);
	}
}

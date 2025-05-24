use crate::prelude::*;
use anyhow::Result;
use beet_common::prelude::*;
use beet_rsx_combinator::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use send_wrapper::SendWrapper;
use sweet::prelude::*;
use syn::Block;
use syn::Expr;
use syn::ExprBlock;

pub fn rsx_to_bundle(
	tokens: &str,
	source_file: WorkspacePathBuf,
) -> Result<TokenStream> {
	TokensApp::with(|app| {
		let entity = app
			.world_mut()
			.spawn((
				SourceFile::new(source_file),
				NodeTokensToBundle::default().exclude_errors(),
				RsxToNodeTokens(tokens.to_string()),
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



#[derive(Default, Component, Deref, Reflect)]
#[reflect(Default, Component)]
pub struct RsxToNodeTokens(pub String);


pub fn rsx_to_node_tokens_plugin(app: &mut App) {
	app.add_systems(Update, rsx_to_node_tokens.in_set(ImportNodesStep));
}


fn rsx_to_node_tokens(
	mut commands: Commands,
	query: Populated<(Entity, &RsxToNodeTokens, Option<&SourceFile>)>,
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
		commands.entity(entity).remove::<RsxToNodeTokens>();
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
		rsx: &RsxToNodeTokens,
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
		let children = self.rsx_parsed_expression(expr)?;
		self.commands
			.entity(root)
			.insert(ItemOf::<(), _>::new(FileSpan::new_for_file(
				&self.source_file,
			)))
			.add_children(&children);
		Ok(())
	}

	// not ideal but we dont have spans for beet_rsx_combinator yet
	fn default_file_span(&self) -> FileSpan {
		FileSpan::new_for_file(&self.source_file)
	}

	fn rsx_parsed_expression(
		&mut self,
		expr: RsxParsedExpression,
	) -> Result<Vec<Entity>> {
		expr.inner()
			.into_iter()
			.map(|item| self.rsx_tokens_or_element(item))
			.collect()
	}

	fn rsx_tokens_or_element(
		&mut self,
		tokens: RsxTokensOrElement,
	) -> Result<Entity> {
		match tokens {
			RsxTokensOrElement::Tokens(tokens) => {
				// TODO this is incorrect, what we need is a new type,
				// like a PartialBlock or something that allows for interspersed
				// tokens and elements
				let block: Block = syn::parse_str(&format!("{{{}}}", &tokens))
					.map_err(|e| {
						anyhow::anyhow!(
							"\nWarning: This parser is a wip so the error may not be accurate \n\
									Failed to parse block:\nblock:{}\nerror:{}",
							tokens,
							e.to_string()
						)
					})?;
				let tracker = self.rusty_tracker.next_tracker(&block);
				let expr = SendWrapper::new(Expr::Block(ExprBlock {
					attrs: Vec::new(),
					label: None,
					block,
				}));
				self.commands
					.spawn((
						BlockNode,
						ItemOf::<BlockNode, _>::new(self.default_file_span()),
						ItemOf::<BlockNode, _>::new(tracker),
						ItemOf::<BlockNode, _>::new(expr),
					))
					.id()
					.xok()
			}
			RsxTokensOrElement::Element(el) => self.rsx_element(el),
		}
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
				.into_iter()
				.flatten()
				.collect::<Vec<_>>()
				.xok()
		}
	}
	fn rsx_child(&mut self, child: RsxChild) -> Result<Vec<Entity>> {
		match child {
			RsxChild::Element(el) => vec![self.rsx_element(el)?].xok(),
			RsxChild::Text(text) => vec![self.rsx_text(text)?].xok(),
			RsxChild::CodeBlock(code_block) => {
				self.rsx_parsed_expression(code_block)
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
				let children = self.rsx_parsed_expression(value)?;
				self.commands
					.spawn((
						AttributeOf::new(parent),
						ItemOf::<AttributeExpr, _>::new(
							self.default_file_span(),
						),
						// AttributeExpr::new(Expr::Block(ExprBlock {
						// 	attrs: Vec::new(),
						// 	label: None,
						// 	block,
						// })),
					))
					.add_children(&children);
			}
			RsxAttribute::Named(name, value) => {
				let key = name.to_string();

				let (val_lit, val_expr, children) =
					self.rsx_attribute_value(value)?;


				let mut entity = self.commands.spawn((
					AttributeOf::new(parent),
					ItemOf::<AttributeOf, _>::new(self.default_file_span()),
					AttributeLit::new(key.clone(), val_lit),
				));
				if !children.is_empty() {
					entity.add_children(&children);
				}
				if let Some(expr) = val_expr {
					entity.insert(AttributeValueExpr::new(expr));
				}
			}
		}
		.xok()
	}
	fn rsx_attribute_value(
		&mut self,
		value: RsxAttributeValue,
	) -> Result<(Option<String>, Option<Expr>, Vec<Entity>)> {
		match value {
			RsxAttributeValue::Default => (None, None, Vec::default()),
			RsxAttributeValue::Boolean(val) => {
				let val = val.0;
				(
					Some(val.to_string()),
					Some(syn::parse_quote!(#val)),
					Vec::default(),
				)
			}
			RsxAttributeValue::Number(val) => {
				let val = val.0;
				(
					Some(val.to_string()),
					Some(syn::parse_quote!(#val)),
					Vec::default(),
				)
			}
			RsxAttributeValue::Str(val) => {
				let val = val.to_string();
				(
					Some(val.to_string()),
					Some(syn::parse_quote!(#val)),
					Vec::default(),
				)
			}
			RsxAttributeValue::Element(value) => {
				let child = self.rsx_element(value)?;
				(None, None, vec![child])
			}
			RsxAttributeValue::CodeBlock(value) => {
				let children = self.rsx_parsed_expression(value)?;
				(None, None, children)
			}
		}
		.xok()
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	fn parse(str: &str) -> Matcher<String> {
		rsx_to_bundle(str, WorkspacePathBuf::new(file!()))
			.unwrap()
			.to_string()
			.xpect()
	}

	#[test]
	fn element() { "<br/>".xmap(parse).to_contain("Element(tag:\"br\""); }
	#[test]
	#[ignore]
	fn unclosed() {
		"<div align=\"center\" />"
			.xmap(parse)
			.to_contain("Element(tag:\"br\"");
	}

	#[test]
	fn text() {
		"<div>hello</div>"
			.xmap(parse)
			.to_contain("Text(value:\"hello\"");
	}
	#[test]
	fn attributes() {
		// default
		"<br foo />".xmap(parse).to_contain("Key(key:\"foo\")");
		// string
		"<br foo=\"bar\"/>"
			.xmap(parse)
			.to_contain("KeyValue(key:\"foo\",value:\"bar\")");
		// bool
		"<br foo=true />"
			.xmap(parse)
			.to_contain("KeyValue(key:\"foo\",value:\"true\")");
		// number
		"<br foo=20 />"
			.xmap(parse)
			.to_contain("KeyValue(key:\"foo\",value:\"20\")");
		// ident
		"<br foo={bar} />"
			.xmap(parse)
			.to_contain("BlockValue(key:\"foo\",tracker:(index:1,");
		// element
		"<br foo={<br/>} />"
			.xmap(parse)
			.to_contain("BlockValue(key:\"foo\",tracker:(index:0,");
	}
}

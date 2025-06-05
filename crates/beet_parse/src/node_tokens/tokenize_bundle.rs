use crate::prelude::*;
use beet_common::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use send_wrapper::SendWrapper;
use sweet::prelude::PipelineTarget;
use syn::Expr;


/// Marker component to be swapped out for a [`BundleTokens`],
/// containing the rust tokens for the node.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
#[component(storage = "SparseSet")]
pub struct GetBundleTokens {
	/// whether parsing errors should be excluded from the output.
	exclude_errors: bool,
}
impl GetBundleTokens {
	pub fn include_errors(mut self) -> Self {
		self.exclude_errors = false;
		self
	}
	pub fn exclude_errors(mut self) -> Self {
		self.exclude_errors = true;
		self
	}
}

/// A [`TokenStream`] representing a [`Bundle`], like a tuple of components.
#[derive(Debug, Clone, Deref, DerefMut, Component)]
pub struct BundleTokens(pub SendWrapper<TokenStream>);
impl BundleTokens {
	pub fn new(value: TokenStream) -> Self { Self(SendWrapper::new(value)) }
	pub fn take(self) -> TokenStream { self.0.take() }
}

pub fn tokenize_bundle_plugin(app: &mut App) {
	app.add_systems(
		Update,
		(resolve_attribute_values, tokenize_bundle_system)
			.chain()
			.in_set(ExportNodesStep),
	);
}


/// the rstml macro parses in steps, ie <div foo={rsx!{<bar/>}}/> will resolve
/// the `bar` node first.
/// the combinator, however, represents attribute value expressions as child nodes
/// ie `<div foo={<bar/>}/>` so we need to resolve the attribute values
/// before walking the node tree.
pub(super) fn resolve_attribute_values(
	_: TempNonSendMarker,
	mut commands: Commands,
	builder: TokenizeBundle,
	attribute_values: Populated<Entity, (With<AttributeOf>, With<Children>)>,
) -> Result {
	for entity in attribute_values.iter() {
		let tokens = builder.tokenize_bundle(entity)?;
		// if parse2 becomes problematic use Expr::Verbatim(tokens)
		let expr = syn::parse2::<Expr>(tokens)?;
		commands
			.entity(entity)
			.insert(AttributeValueExpr::new(expr));
	}

	Ok(())
}


/// Walks children of an entity collecting into a [`BundleTokens`].
// TODO i guess this will be a bottleneck, challenging as TokenStream is not `Send`
fn tokenize_bundle_system(
	_: TempNonSendMarker,
	mut commands: Commands,
	tokenizer: TokenizeBundle,
	template_roots: Populated<(
		Entity,
		&GetBundleTokens,
		Option<&TokensDiagnostics>,
	)>,
) -> Result {
	for (entity, settings, diagnostics) in template_roots.iter() {
		let mut tokens = tokenizer.tokenize_child_bundles(entity)?;
		if !settings.exclude_errors
			&& let Some(diagnostics) = diagnostics
		{
			let diagnostics = TokensDiagnostics((*diagnostics).clone());
			tokens.extend(diagnostics.into_tokens());
		}
		commands.entity(entity).insert(BundleTokens::new(tokens));
	}
	Ok(())
}


/// recursively visit children and collect into a [`TokenStream`].
/// We use a custom [`SystemParam`] for the traversal, its more of
/// a 'map' function than an 'iter', as we need to resolve children
/// and then wrap them as `children![]` in parents.
#[derive(SystemParam)]
pub(super) struct TokenizeBundle<'w, 's> {
	children: TokenizeRelated<'w, 's, Children>,
	// children: Query<'w, 's, &'static Children>,
	block_node_exprs:
		Query<'w, 's, &'static ItemOf<BlockNode, SendWrapper<Expr>>>,
	combinator_exprs: Query<'w, 's, &'static CombinatorExpr>,
	rsx_nodes: TokenizeRsxNode<'w, 's>,
	rsx_directives: TokenizeRsxDirectives<'w, 's>,
	web_nodes: TokenizeWebNodes<'w, 's>,
	web_directives: TokenizeWebDirectives<'w, 's>,
	node_attributes: TokenizeAttributes<'w, 's>,
}

impl TokenizeBundle<'_, '_> {
	/// Entry point for the builder, rstml token roots are not elements themselves,
	/// so if theres only one child return that instead of a fragment
	fn tokenize_child_bundles(&self, entity: Entity) -> Result<TokenStream> {
		let Some(children) = self.children.get(entity).ok() else {
			return Ok(quote! { () });
		};
		if children.len() == 1 {
			// a single child, return that
			self.tokenize_bundle(children[0])
		} else {
			// multiple children, wrap in fragment
			let children = children
				.iter()
				.map(|child| self.tokenize_bundle(child))
				.collect::<Result<Vec<_>>>()?;
			Ok(quote! { (
				FragmentNode,
				children![#(#children),*])
			})
		}
	}


	fn tokenize_bundle(&self, entity: Entity) -> Result<TokenStream> {
		let mut items = Vec::<TokenStream>::new();
		self.rsx_nodes.tokenize_components(&mut items, entity)?;
		self.rsx_directives
			.tokenize_components(&mut items, entity)?;
		self.web_nodes.tokenize_components(&mut items, entity)?;
		self.web_directives
			.tokenize_components(&mut items, entity)?;
		self.node_attributes.try_push_attributes(
			|e| self.tokenize_combinator_expr(e),
			&mut items,
			entity,
		)?;
		self.tokenize_block_node_exprs(&mut items, entity)?;
		self.tokenize_combinator_exprs(&mut items, entity)?;
		self.children
			.try_push_related(&mut items, entity, |child| {
				self.tokenize_bundle(child)
			})?;

		if items.is_empty() {
			// no components, unit type
			quote! { () }
		} else if items.len() == 1 {
			// a single components
			items.pop().unwrap()
		} else {
			// a component tuple
			quote! {(#(#items),*) }
		}
		.xok()
	}
	fn tokenize_block_node_exprs(
		&self,
		items: &mut Vec<TokenStream>,
		entity: Entity,
	) -> Result<()> {
		if let Ok(block) = self.block_node_exprs.get(entity) {
			let block = &***block;
			items.push(quote! {#block.into_node_bundle()});
		}
		Ok(())
	}
	/// push combinators for nodes, attributes are handled by CollectNodeAttributes
	fn tokenize_combinator_exprs(
		&self,
		items: &mut Vec<TokenStream>,
		entity: Entity,
	) -> Result<()> {
		if let Some(expr) = self.tokenize_combinator_expr(entity)? {
			items.push(quote! {#expr});
		}
		Ok(())
	}
	fn tokenize_combinator_expr(
		&self,
		entity: Entity,
	) -> Result<Option<TokenStream>> {
		if let Ok(combinator) = self.combinator_exprs.get(entity) {
			let mut expr = String::new();
			for item in combinator.iter() {
				match item {
					CombinatorExprPartial::Tokens(tokens) => {
						expr.push_str(tokens);
					}
					CombinatorExprPartial::Element(entity) => {
						let tokens = self.tokenize_bundle(*entity)?;
						expr.push_str(&tokens.to_string());
					}
				}
			}
			// combinator removes braces so we put them back
			let expr = format!("{{{}}}", expr);
			let expr_tokens = syn::parse_str::<TokenStream>(&expr)?;
			return Ok(Some(expr_tokens));
		} else {
			Ok(None)
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_common::prelude::*;
	use bevy::prelude::*;
	use quote::quote;
	use sweet::prelude::*;

	#[test]
	fn works() {
		quote! {
			<span
				{EntityObserver::new(|_on_click:Trigger<OnClick>|{})}
				hidden=true
				onmousemove="some_js_func"
				onclick=|| { println!("clicked"); }
				>
				<MyComponent foo="bar" client:load />
				<div/>
			</span>
		}
		.xmap(|t| rstml_to_bundle(t, WorkspacePathBuf::new(file!())))
		.unwrap()
		.to_string()
		.xpect()
		.to_be(
			quote! {(
				NodeTag(String::from("span")),
				ElementNode { self_closing: false },
				{ EntityObserver::new(|_on_click: Trigger<OnClick>| {}) }.into_node_bundle(),
				EntityObserver::new(#[allow(unused_braces)] |_: Trigger<OnClick>| { println!("clicked"); }),
				related!(Attributes[(
					"hidden".into_attr_key_bundle(),
					true.into_attr_val_bundle()
				), (
					"onmousemove".into_attr_key_bundle(),
					"some_js_func".into_attr_val_bundle()
				)]),
				related!{Children[(
					NodeTag(String::from("MyComponent")),
					FragmentNode,
					ClientIslandDirective::Load,
					ItemOf::<beet_common::node::rsx_nodes::TemplateNode, beet_common::templating::rusty_tracker::RustyTracker> {
						value: RustyTracker { index: 0u32, tokens_hash: 6523630531850795118u64 },
						phantom: std::marker::PhantomData::<beet_common::node::rsx_nodes::TemplateNode>
					},
					{
						let template = <MyComponent as Props>::Builder::default().foo("bar").build();
						#[allow(unused_braces)]
						(
							#[cfg(not(target_arch = "wasm32"))]
							{ TemplateSerde::new(&template) }
							#[cfg(target_arch = "wasm32")]
							{ () },
							TemplateRoot::spawn(Spawn(template.into_node_bundle()))
						)
					}
				), (
					NodeTag(String::from("div")),
					ElementNode { self_closing: true }
				)]}
			)}
			.to_string(),
		);
	}

	#[test]
	fn multiple_root_children() {
		quote! {
			<br/>
			<br/>
		}
		.xmap(|t| rstml_to_bundle(t, WorkspacePathBuf::new(file!())))
		.unwrap()
		.to_string()
		.xpect()
		.to_be(
			quote! {
				(
					FragmentNode,
					children![
						(
							NodeTag(String::from("br")),
							ElementNode { self_closing: true }
						),
						(
							NodeTag(String::from("br")),
							ElementNode { self_closing: true }
						)
					]
				)
			}
			.to_string(),
		);
	}
	#[test]
	fn blocks() {
		quote! {{foo}}
			.xmap(|t| rstml_to_bundle(t, WorkspacePathBuf::new(file!())))
			.unwrap()
			.to_string()
			.xpect()
			.to_be(
				quote! {(
					BlockNode,
					{foo}.into_node_bundle()
				)}
				.to_string(),
			);
	}
	#[test]
	fn attribute_blocks() {
		quote! {<input hidden=val/>}
			.xmap(|t| rstml_to_bundle(t, WorkspacePathBuf::new(file!())))
			.unwrap()
			.to_string()
			.xpect()
			.to_be(
				quote! {(
					NodeTag(String::from("input")),
					ElementNode { self_closing: true },
					related!(Attributes [
						(
							"hidden".into_attr_key_bundle(),
							val.into_attr_val_bundle()
						)
					])
				)}
				.to_string(),
			);
	}

	// copy paste from above test to see if the tokens are a valid bundle
	#[test]
	fn output_check() {
		World::new().spawn(
			// start copy pasta
			children![(
				NodeTag(String::from("span")),
				ElementNode {
					self_closing: false
				},
				{ EntityObserver::new(|_on_click: Trigger<OnClick>| {}) },
				EntityObserver::new(|_: Trigger<OnClick>| {
					println!("clicked");
				}),
				related!(Attributes [
					(
						AttributeKey::new("hidden"),
						AttributeValue::new(true),
						AttributeKeyStr(String::from("hidden")),
						AttributeValueStr(String::from("true"))
					),
					(
						AttributeKey::new("onmousemove"),
						AttributeValue::new("some_js_func"),
						AttributeKeyStr(String::from("onmousemove")),
						AttributeValueStr (String::from("some_js_func"))
					)
				]),
				children![
					(
						NodeTag(String::from("MyComponent")),
						ClientIslandDirective::Load
					),
					(NodeTag(String::from("div")), ElementNode {
						self_closing: true
					})
				]
			)], // end copy pasta
		);
	}
}

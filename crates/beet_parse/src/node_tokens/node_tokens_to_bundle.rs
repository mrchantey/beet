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
pub struct NodeTokensToBundle {
	/// whether parsing errors should be excluded from the output.
	exclude_errors: bool,
}
impl NodeTokensToBundle {
	pub fn include_errors(mut self) -> Self {
		self.exclude_errors = false;
		self
	}
	pub fn exclude_errors(mut self) -> Self {
		self.exclude_errors = true;
		self
	}
}


pub fn node_tokens_to_bundle_plugin(app: &mut App) {
	app.add_systems(Update, node_tokens_to_bundle.in_set(ExportNodesStep));
}

/// A [`TokenStream`] representing a bevy bundle, usually a tuple.
#[derive(Debug, Clone, Deref, DerefMut, Component)]
pub struct BundleTokens(pub SendWrapper<TokenStream>);
impl BundleTokens {
	pub fn new(value: TokenStream) -> Self { Self(SendWrapper::new(value)) }
	pub fn take(self) -> TokenStream { self.0.take() }
}

/// Walks children of an entity collecting into a [`BundleTokens`].
// TODO i guess this will be a bottleneck, challenging as TokenStream is not `Send`
fn node_tokens_to_bundle(
	_: TempNonSendMarker,
	mut commands: Commands,
	builder: Builder,
	template_roots: Populated<(
		Entity,
		&NodeTokensToBundle,
		Option<&TokensDiagnostics>,
	)>,
) -> Result {
	for (entity, settings, diagnostics) in template_roots.iter() {
		let mut tokens = builder.token_stream_from_root(entity)?;
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
struct Builder<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	rsx_nodes: CollectRsxNodeTokens<'w, 's>,
	block_node_exprs:
		Query<'w, 's, &'static ItemOf<BlockNode, SendWrapper<Expr>>>,
	rsx_directives: CollectRsxDirectiveTokens<'w, 's>,
	web_nodes: CollectWebNodeTokens<'w, 's>,
	web_directives: CollectWebDirectiveTokens<'w, 's>,
	node_attributes: CollectNodeAttributes<'w, 's>,
}

impl Builder<'_, '_> {
	/// Entry point for the builder, rstml token roots are not elements themselves,
	/// so if theres only one child return that instead of a fragment
	fn token_stream_from_root(&self, entity: Entity) -> Result<TokenStream> {
		let Some(children) = self.children.get(entity).ok() else {
			return Ok(quote! { () });
		};
		if children.len() == 1 {
			// a single child, return that
			self.token_stream(children[0])
		} else {
			// multiple children, wrap in children![]
			let children = children
				.iter()
				.map(|child| self.token_stream(child))
				.collect::<Result<Vec<_>>>()?;
			Ok(quote! { (
				FragmentNode,
				children![#(#children),*])
			})
		}
	}


	fn token_stream(&self, entity: Entity) -> Result<TokenStream> {
		let mut items = Vec::<TokenStream>::new();
		self.rsx_nodes.try_push_all(&mut items, entity)?;
		self.rsx_directives.try_push_all(&mut items, entity)?;
		self.web_nodes.try_push_all(&mut items, entity)?;
		self.web_directives.try_push_all(&mut items, entity)?;
		self.node_attributes.try_push_all(&mut items, entity)?;
		if let Ok(block) = self.block_node_exprs.get(entity) {
			let block = &***block;
			items.push(quote! {#block.into_node_bundle()});
		}


		if let Ok(children) = self.children.get(entity) {
			let children = children
				.iter()
				.map(|child| self.token_stream(child))
				.collect::<Result<Vec<_>>>()?;
			if !children.is_empty() {
				items.push(quote! { children![#(#children),*] });
			}
		};

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
		.xmap(|t| rstml_tokens_to_rust(t, WorkspacePathBuf::new(file!())))
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
				children![(
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
							children![template.into_node_bundle()]
						)
					}
				), (
					NodeTag(String::from("div")),
					ElementNode { self_closing: true }
				)]
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
		.xmap(|t| rstml_tokens_to_rust(t, WorkspacePathBuf::new(file!())))
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
			.xmap(|t| rstml_tokens_to_rust(t, WorkspacePathBuf::new(file!())))
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
			.xmap(|t| rstml_tokens_to_rust(t, WorkspacePathBuf::new(file!())))
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

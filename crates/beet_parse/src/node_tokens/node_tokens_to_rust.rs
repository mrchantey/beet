use crate::prelude::*;
use beet_common::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use sweet::prelude::PipelineTarget;


/// Marker component to be swapped out for a [`NonSendHandle<TokenStream>`],
/// containing the rust tokens for the node.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct NodeTokensToRust;


pub fn node_tokens_to_rust_plugin(app: &mut App) {
	app.init_non_send_resource::<NonSendAssets<TokenStream>>()
		.add_systems(Update, node_tokens_to_rust.in_set(ExportNodesStep));
}


/// Walks children of an entity collecting into an [`impl Bundle`] [`TokenStream`].
// TODO i guess this will be a bottleneck, challenging as TokenStream is not `Send`
fn node_tokens_to_rust(
	mut commands: Commands,
	mut token_streams: NonSendMut<NonSendAssets<TokenStream>>,
	builder: Builder,
	template_roots: Populated<Entity, With<NodeTokensToRust>>,
) -> Result {
	for entity in template_roots.iter() {
		let tokens = builder.token_stream(entity)?;
		commands.entity(entity).insert(token_streams.insert(tokens));
	}
	Ok(())
}

/// recursively visit children and collect into a [`TokenStream`]
#[derive(SystemParam)]
struct Builder<'w, 's> {
	spans: NonSend<'w, NonSendAssets<Span>>,
	children: Query<'w, 's, &'static Children>,
	rsx_nodes: CollectRsxNodeTokens<'w, 's>,
	rsx_directives: CollectRsxDirectiveTokens<'w, 's>,
	web_nodes: CollectWebNodeTokens<'w, 's>,
	web_directives: CollectWebDirectiveTokens<'w, 's>,
	node_attributes: CollectNodeAttributes<'w, 's>,
}

impl Builder<'_, '_> {
	fn token_stream(&self, entity: Entity) -> Result<TokenStream> {
		let mut items = Vec::<TokenStream>::new();
		self.rsx_nodes
			.try_push_all(&self.spans, &mut items, entity)?;
		self.rsx_directives
			.try_push_all(&self.spans, &mut items, entity)?;
		self.web_nodes
			.try_push_all(&self.spans, &mut items, entity)?;
		self.web_directives
			.try_push_all(&self.spans, &mut items, entity)?;
		self.node_attributes
			.try_push_all(&self.spans, &mut items, entity)?;

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
	use proc_macro2::TokenStream;
	use quote::quote;
	use sweet::prelude::*;

	fn parse(tokens: TokenStream) -> TokenStream {
		App::new()
			.add_plugins(NodeTokensPlugin)
			.xtap(|app| {
				app.world_mut()
					.spawn((
						SourceFile::new(WorkspacePathBuf::new(file!())),
						NodeTokensToRust,
					))
					.insert_non_send(RstmlTokens::new(tokens));
			})
			.update_then()
			.xmap(|app| {
				app.world_mut()
					.remove_non_send_resource::<NonSendAssets<TokenStream>>()
					.unwrap()
					.into_inner()
					.into_values()
					.next()
					.unwrap()
			})
	}


	#[test]
	fn works() {
		quote! {
			<span
				// {|on_click:Trigger<OnClick>|{}}
				hidden=true
				onclick=|_| { println!("clicked"); }
				>
				<MyComponent client:load />
				<div/>
			</span>
		}
		.xmap(parse)
		.to_string()
		.xpect()
		.to_be(
			quote! {
			children![(
						NodeTag(String::from("span")),
						ElementNode {
							self_closing: false
						},
						EntityObserver::new::<OnClick,_,_,_>(|_|{println!("clicked") ; }),
						related!(Attributes [
							(
								AttributeKey::new("hidden"),
								AttributeValue::new(true),
								AttributeKeyStr(String::from("hidden")),
								AttributeValueStr(String::from("true"))
							),
							(
								AttributeKey::new("onclick"),
								AttributeKeyStr(String::from("onclick"))
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
					)]
				}
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
						AttributeKey::new("onclick"),
						AttributeKeyStr(String::from("onclick"))
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

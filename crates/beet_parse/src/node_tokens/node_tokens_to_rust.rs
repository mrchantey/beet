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
		println!("here");
		let tokens = builder.token_stream(entity)?;
		commands.entity(entity).insert(token_streams.insert(tokens));
	}
	Ok(())
}

/// recursively visit children and collect into a [`TokenStream`]
#[derive(SystemParam)]
struct Builder<'w, 's> {
	spans: NonSend<'w, NonSendAssets<Span>>,
	doctypes: MaybeWithItem<'w, 's, DoctypeNode, NonSendHandle<Span>>,
	comments: MaybeWithItem<'w, 's, CommentNode, NonSendHandle<Span>>,
	elements: MaybeWithItem<'w, 's, ElementNode, NonSendHandle<Span>>,
	children: Query<'w, 's, &'static Children>,
}

type MaybeWithItem<'w, 's, C, T> =
	Query<'w, 's, (&'static C, Option<&'static T>)>;

impl Builder<'_, '_> {
	fn token_stream(&self, entity: Entity) -> Result<TokenStream> {
		let Ok(children) = self.children.get(entity) else {
			return TokenStream::new().xok();
		};

		let mut items = Vec::<TokenStream>::new();
		if let Ok(doctypes) = self.doctypes.get(entity) {
			items.push(self.to_tokens_maybe_spanned(doctypes)?);
		}
		if let Ok(comments) = self.comments.get(entity) {
			items.push(self.to_tokens_maybe_spanned(comments)?);
		}
		if let Ok(elements) = self.elements.get(entity) {
			items.push(self.to_tokens_maybe_spanned(elements)?);
		}

		let children = children
			.iter()
			.map(|child| self.token_stream(child))
			.collect::<Result<Vec<_>>>()?;
		if !children.is_empty() {
			items.push(quote! { children![#(#children),*] });
		}


		if items.is_empty() {
			TokenStream::new()
		} else if items.len() == 1 {
			items.pop().unwrap()
		} else {
			quote! {#(#items),* }
		}
		.xok()
	}
	fn to_tokens_maybe_spanned<T: IntoCustomTokens>(
		&self,
		(item, span): (&T, Option<&NonSendHandle<Span>>),
	) -> Result<TokenStream> {
		if let Some(span) = span {
			let span = *self.spans.get(span)?;
			let item = item.into_custom_token_stream();
			quote::quote_spanned! { span =>
				#item
			}
		} else {
			item.into_custom_token_stream()
		}
		.xok()
	}
}


#[cfg(test)]
mod test {
	use std::str::FromStr;

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
		let mut tokens = TokenStream::new();
		let inner = true.into_custom_token_stream();
		tokens.extend(quote::quote! {ElementNode {#inner}});
		// tokens.extend(TokenStream::from_str("{"));
		// tokens.extend(TokenStream::from_str("}"));
		// tokens.extend(quote::quote! {self_closing: });
		// tokens.extend(true.into_custom_token_stream());

		let a = tokens.to_string();
		println!("tokens: {a}");
		// quote! {
		// 	<span>
		// 		<MyComponent client:load />
		// 		<div/>
		// 	</span>
		// }
		// .xmap(parse)
		// .to_string()
		// .xpect()
		// .to_be(
		// 	quote! {
		// 		span(
		// 			"client:load",
		// 			"div",
		// 		)
		// 	}
		// 	.to_string(),
		// );
	}
}

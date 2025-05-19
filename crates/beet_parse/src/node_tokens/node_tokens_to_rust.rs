use crate::prelude::*;
use beet_common::node::DoctypeNode;
use beet_common::prelude::NonSendAssets;
use beet_common::prelude::NonSendHandle;
use beet_common::prelude::RustTokens;
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
	app.add_systems(Update, node_tokens_to_rust.in_set(ExportNodesStep));
}


/// Walks children of an entity collecting into an [`impl Bundle`] [`TokenStream`].
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
	doctypes: MaybeWithItem<'w, 's, DoctypeNode, NonSendHandle<Span>>,
	children: Query<'w, 's, &'static Children>,
}

impl Builder<'_, '_> {
	fn token_stream(&self, entity: Entity) -> Result<TokenStream> {
		let Ok(children) = self.children.get(entity) else {
			return TokenStream::new().xok();
		};

		let mut items = Vec::<TokenStream>::new();
		if let Ok(doctypes) = self.doctypes.get(entity) {
			self.to_tokens_maybe_spanned(doctypes)?;
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
	fn to_tokens_maybe_spanned<T: RustTokens>(
		&self,
		(item, span): (&T, Option<&NonSendHandle<Span>>),
	) -> Result<TokenStream> {
		if let Some(span) = span {
			let span = *self.spans.get(span)?;
			let item = item.into_rust_tokens();
			quote::quote_spanned! { span =>
				#item
			}
		} else {
			item.into_rust_tokens()
		}
		.xok()
	}
}


type MaybeWithItem<'w, 's, C, T> =
	Query<'w, 's, (&'static C, Option<&'static T>)>;

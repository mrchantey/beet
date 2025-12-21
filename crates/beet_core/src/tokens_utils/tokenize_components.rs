use crate::prelude::*;
use beet_core_macros::ToTokens;
use proc_macro2::TokenStream;
use variadics_please::all_tuples;

pub trait TokenizeComponents {
	fn tokenize_if_present(
		world: &World,
		items: &mut Vec<TokenStream>,
		entity: Entity,
	);
}

impl<T> TokenizeComponents for T
where
	T: Component + TokenizeSelf,
{
	fn tokenize_if_present(
		world: &World,
		items: &mut Vec<TokenStream>,
		entity: Entity,
	) {
		let entity_ref = world.entity(entity);
		match (entity_ref.get::<T>(), entity_ref.get::<SpanOf<T>>()) {
			(Some(value), Some(span)) => {
				let value = value.self_token_stream();
				items.push(quote::quote_spanned! { **span =>
					#value
				})
			}
			(Some(value), None) => items.push(value.self_token_stream()),
			_ => {}
		};
	}
}


macro_rules! impl_tokenize_components_tuple {
	($(($T:ident, $t:ident)),*) => {
		impl<$($T),*> TokenizeComponents for ($($T,)*)
		where
			$($T: TokenizeComponents,)*
		{
			fn tokenize_if_present(
				world: &World,
				items: &mut Vec<TokenStream>,
				entity: Entity,
			) {
				$(
					<$T as TokenizeComponents>::tokenize_if_present(world, items, entity);
				)*
			}
		}
	}
}

all_tuples!(impl_tokenize_components_tuple, 1, 15, T, t);

#[derive(Debug, Clone, Component, ToTokens)]
pub struct SpanOf<C> {
	pub value: send_wrapper::SendWrapper<proc_macro2::Span>,
	pub phantom: std::marker::PhantomData<C>,
}


impl<C> std::ops::Deref for SpanOf<C> {
	type Target = proc_macro2::Span;
	fn deref(&self) -> &Self::Target { &self.value }
}

impl<C> SpanOf<C> {
	pub fn new(value: proc_macro2::Span) -> Self {
		Self {
			value: send_wrapper::SendWrapper::new(value),
			phantom: std::marker::PhantomData,
		}
	}
	pub fn take(self) -> proc_macro2::Span { self.value.take() }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[derive(Component, ToTokens)]
	struct Foo;
	#[derive(Component, ToTokens)]
	struct Bar;

	#[test]
	fn works() {
		let mut world = World::new();
		let entity = world.spawn((Foo, Bar)).id();

		type List = (Foo, Bar);

		let mut items = Vec::new();
		// Test tuple implementation for (ElementNode, NodeTag)
		List::tokenize_if_present(&world, &mut items, entity);

		// Should have two token streams
		items.len().xpect_eq(2);
		// check that the token streams are not empty
		items.iter().all(|ts| !ts.is_empty()).xpect_true();
	}
}

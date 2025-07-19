use crate::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use variadics_please::all_tuples;


pub trait TokenizeComponents {
	fn tokenize_if_present(
		world: &World,
		items: &mut Vec<TokenStream>,
		entity: Entity,
	);
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


// a group of all groups of components
pub type RsxComponents = (
	RootComponents,
	RsxNodes,
	WebNodes,
	RsxDirectives,
	WebDirectives,
);

type RootComponents = (
	SnippetRoot,
	StaticRoot,
	InstanceRoot,
	ResolvedRoot,
	ExprIdx,
	RequiresDomIdx,
);

type RsxNodes = (NodeTag, FragmentNode, TemplateNode, TextNode, BlockNode);

type WebNodes = (DoctypeNode, CommentNode, ElementNode);

type RsxDirectives = (SlotChild, SlotTarget);

type WebDirectives = (
	HtmlHoistDirective,
	ClientLoadDirective,
	ClientOnlyDirective,
	StyleScope,
	StyleCascade,
	ScriptElement,
	StyleElement,
	CodeElement,
	InnerText,
);



#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();
		let entity =
			world.spawn((ElementNode::open(), NodeTag::new("div"))).id();

		let mut items = Vec::new();
		// Test tuple implementation for (ElementNode, NodeTag)
		RsxComponents::tokenize_if_present(&world, &mut items, entity);

		// Should have two token streams
		expect(items.len()).to_be(2);
		// check that the token streams are not empty
		expect(items.iter().all(|ts| !ts.is_empty())).to_be_true();
	}
}

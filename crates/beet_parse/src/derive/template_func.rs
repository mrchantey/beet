use crate::prelude::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use syn::ItemFn;
use syn::Result;

pub fn template_func(input: ItemFn) -> TokenStream {
	parse(input).unwrap_or_else(|err| err.into_compile_error())
}

fn parse(input: ItemFn) -> Result<TokenStream> {
	let fields = NodeField::parse_item_fn(&input)?;
	let define_struct = define_struct(&input, &fields)?;
	let impl_template_bundle = impl_template_bundle(&input, &fields)?;

	Ok(quote! {
		use beet::prelude::*;
		#define_struct
		#impl_template_bundle
		// #impl_props
		// #impl_builder
		// #impl_required
		// #impl_flatten
	})
}

fn define_struct(func: &ItemFn, fields: &[NodeField]) -> Result<TokenStream> {
	let attrs = &func.attrs;

	let (_, type_generics, where_clause) = func.sig.generics.split_for_impl();
	let ident = &func.sig.ident;

	let fields = fields.iter().map(|f| {
		let ident = &f.ident;
		let attrs = &f.attrs;
		let ty = f.ty;
		quote! {
			#(#attrs)*
			pub #ident: #ty
		}
	});
	let vis = &func.vis;

	Ok(quote! {
	#(#attrs)*
	#[derive(Props)]
	#vis struct #ident #type_generics #where_clause {
		#(#fields),*
	}
	})
}

fn impl_template_bundle(
	func: &ItemFn,
	fields: &[NodeField],
) -> Result<TokenStream> {
	let (impl_generics, type_generics, where_clause) =
		func.sig.generics.split_for_impl();
	let ident = &func.sig.ident;

	let destructure = fields.iter().map(|field| {
		let mutability = field
			.mutability
			.map(|m| m.to_token_stream())
			.unwrap_or_default();
		let ident = &field.ident;
		quote! {
			#mutability #ident
		}
	});
	let body = &func.block.stmts;
	let return_type = &func.sig.output;

	Ok(quote! {
	impl #impl_generics IntoTemplateBundle<Self> for #ident #type_generics #where_clause {
		fn into_node_bundle(self) #return_type {
			let Self{#(#destructure),*} = self;
			#(#body)*
		}
	}
	})
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use quote::quote;
	use sweet::prelude::*;

	#[test]
	fn simple() {
		template_func(syn::parse_quote! {
			/// probably the best templating layout
			pub(crate) fn MyNode(
				/// some comment
				foo:u32,
				mut bar:u32
			) -> impl Bundle{()}
		})
		.to_string()
		.xpect()
		.to_be(
			quote! {
			use beet::prelude::*;
			#[doc = r" probably the best templating layout"]
			#[derive(Props)]
			pub(crate) struct MyNode {
				#[doc = r" some comment"]
				pub foo: u32,
				pub bar: u32
			}
			impl IntoTemplateBundle<Self> for MyNode {
				fn into_node_bundle(self) -> impl Bundle {
					let Self { foo, mut bar } = self;
					()
				}
			}
			}
			.to_string(),
		);
	}
}

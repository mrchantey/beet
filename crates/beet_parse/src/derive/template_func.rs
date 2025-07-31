use crate::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;
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

	let fields = prop_fields(fields).map(|f| {
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

	let destructure_props = prop_fields(fields).map(|field| {
		let mutability = field.mutability;
		let ident = &field.ident;
		quote! {
			#mutability #ident
		}
	});

	let param_fields = system_param_fields(fields).map(|field| {
		let ident = &field.ident;
		let ty = &field.ty;
		let attrs = &field.attrs;
		let mutability = field.mutability;
		quote! {
		#(#attrs)*
		#mutability #ident: #ty
		}
	});

	let body = &func.block.stmts;
	let assign_entity = entity_field(fields)
		.map(|entity| quote! { let #entity = entity_world_mut.id(); });

	let err_msg =
		format!("Error running template system for `{}`: {{}}", ident);

	Ok(quote! {
	impl #impl_generics IntoBundle<Self> for #ident #type_generics #where_clause {
		fn into_bundle(self) -> impl Bundle {
			OnSpawn::new(move |entity_world_mut: &mut EntityWorldMut| {
				#assign_entity
				let bundle = entity_world_mut.world_scope(|world| {
					world.run_system_cached_with(move |In(input): In<Self>, #(#param_fields),*| {
						// panic!("here");
						let Self { #(#destructure_props),* } = input;
						#(#body)*
					}, self).map_err(|err|
						bevyhow!(#err_msg, err)

					).unwrap_or_exit()
				});
				entity_world_mut.insert(bundle);
			})
		}
	}
	})
}


const SYSTEM_PARAM_IDENTS: [&str; 7] = [
	"World",
	"Commands",
	"Res",
	"ResMut",
	"Query",
	"Populated",
	"When",
];

/// Gets all non system param fields
fn prop_fields<'a>(
	fields: &'a [NodeField],
) -> impl Iterator<Item = &'a NodeField<'a>> {
	fields
		.iter()
		.filter(|f| !f.last_segment_matches("Entity"))
		.filter(|f| {
			!SYSTEM_PARAM_IDENTS
				.iter()
				.any(|id| f.last_segment_matches(id))
		})
}

fn system_param_fields<'a>(
	fields: &'a [NodeField],
) -> impl Iterator<Item = &'a NodeField<'a>> {
	fields
		.iter()
		.filter(|f| !f.last_segment_matches("Entity"))
		.filter(|f| {
			SYSTEM_PARAM_IDENTS
				.iter()
				.any(|id| f.last_segment_matches(id))
		})
}


fn entity_field<'a>(fields: &'a [NodeField]) -> Option<&'a Ident> {
	fields
		.iter()
		.find(|field| field.last_segment_matches("Entity"))
		.map(|field| field.ident)
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;
	use syn::PathSegment;

	#[test]
	fn segments() {
		let a: PathSegment = syn::parse_quote! {Foo};
		expect(a.ident).to_be("Foo");
		let a: PathSegment = syn::parse_quote! {Foo<Bar>};
		expect(a.ident).to_be("Foo");
	}

	#[test]
	fn simple() {
		template_func(syn::parse_quote! {
			/// probably the best templating layout ever
			pub(crate) fn MyNode(
				/// some comment
				foo:u32,
				mut bar:u32
			) -> impl Bundle{()}
		})
		.xpect()
		.to_be_snapshot();
	}
	#[test]
	fn complex() {
		template_func(syn::parse_quote! {
			/// probably the best templating layout ever
			pub(crate) fn MyNode(
				/// some comment
				foo:u32,
				mut bar:u32,
				my_entity:Entity,
				world: &mut World,
				res: Res<Time>,
				mut query: Query<&mut Transform>,
			) -> impl Bundle{()}
		})
		.xpect()
		.to_be_snapshot();
	}
}

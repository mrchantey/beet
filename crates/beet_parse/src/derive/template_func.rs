use crate::prelude::*;
use beet_utils::prelude::pkg_ext;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;
use syn::ItemFn;
use syn::Result;
use syn::ReturnType;

pub fn template_func(input: ItemFn) -> TokenStream {
	parse(input).unwrap_or_else(|err| err.into_compile_error())
}

fn parse(input: ItemFn) -> Result<TokenStream> {
	let fields = NodeField::parse_item_fn(&input)?;
	let define_struct = define_struct(&input, &fields)?;
	let impl_template_bundle = impl_template_bundle(&input, &fields)?;

	let imports = if pkg_ext::is_internal() {
		quote! {
			use bevy::prelude::*;
			use beet_core::prelude::*;
			use beet_utils::prelude::*;
		}
	} else {
		quote! {
			use beet::prelude::*;
		}
	};

	Ok(quote! {
		#imports
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
	let ident = &func.sig.ident;
	let (impl_generics, type_generics, where_clause) =
		func.sig.generics.split_for_impl();

	let return_type = with_captured_lifetimes(&func);

	let body = &func.block.stmts;
	let entity_ident = entity_param_ident(fields)
		.cloned()
		.unwrap_or_else(|| Ident::new("entity", ident.span()));

	let err_msg =
		format!("Error running template system for `{}`: {{}}", ident);


	let returns_result = if let ReturnType::Type(_, ty) = &return_type
		&& let syn::Type::Path(type_path) = &**ty
	{
		type_path
			.path
			.segments
			.last()
			.map_or(false, |segment| segment.ident == "Result")
	} else {
		false
	};
	let maybe_unwrap = if returns_result {
		quote! { .unwrap_or_exit() }
	} else {
		Default::default()
	};

	Ok(quote! {

	impl #impl_generics #ident #type_generics #where_clause {
		pub fn system(#[allow(unused_variables)]In((#entity_ident, input)): In<(Entity,Self)>, #(#param_fields),*) #return_type{
				#[allow(unused_variables)]
				let Self { #(#destructure_props),* } = input;
				#(#body)*
		}
	}

	impl #impl_generics IntoBundle<Self> for #ident #type_generics #where_clause {
		fn into_bundle(self) -> impl Bundle {
			OnSpawn::new(move |entity_world_mut: &mut EntityWorldMut| {
				let id = entity_world_mut.id();
				let bundle = entity_world_mut.world_scope(|world| {
					world.run_system_cached_with(Self::system, (id,self)).map_err(|err|
						bevyhow!(#err_msg, err)
					).unwrap_or_exit()
				})#maybe_unwrap;
				entity_world_mut.insert(bundle.into_bundle());
			})
		}
	}
	})
}

/// Any type in the ReturnType that is an impl will need an additional `use<>`
/// constraint, required for valid bevy systems.
/// https://doc.rust-lang.org/edition-guide/rust-2024/rpit-lifetime-capture.html
fn with_captured_lifetimes(func: &ItemFn) -> ReturnType {
	fn impl_recursive(func: &ItemFn, ty: &mut syn::Type) {
		match ty {
			syn::Type::Path(type_path) => {
				for segment in &mut type_path.path.segments {
					if let syn::PathArguments::AngleBracketed(args) =
						&mut segment.arguments
					{
						for arg in &mut args.args {
							if let syn::GenericArgument::Type(ty) = arg {
								impl_recursive(func, ty);
							}
						}
					}
				}
			}
			syn::Type::ImplTrait(impl_trait) => {
				let bound = if func.sig.generics.params.is_empty() {
					syn::parse_quote! { use<> }
				} else {
					let (_, type_generics, _) =
						func.sig.generics.split_for_impl();
					syn::parse_quote! { use #type_generics }
				};

				impl_trait.bounds.push(bound);
			}
			_ => {}
		}
	}

	let mut return_type = func.sig.output.clone();
	if let ReturnType::Type(_, ty) = &mut return_type {
		impl_recursive(func, &mut *ty);
	}
	return_type
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


fn entity_param_ident<'a>(fields: &'a [NodeField]) -> Option<&'a Ident> {
	fields
		.iter()
		.find(|field| field.last_segment_matches("Entity"))
		.map(|field| field.ident)
}


#[cfg(test)]
mod test {
	use super::with_captured_lifetimes;
	use crate::prelude::*;
	use sweet::prelude::*;
	use syn::PathSegment;

	#[test]
	fn capture_lifetimes_test() {
		with_captured_lifetimes(&syn::parse_quote! {
			fn foo<T>() -> impl Bundle {}
		})
		.xpect_eq(syn::parse_quote! {-> impl Bundle + use<T> });

		with_captured_lifetimes(
			&syn::parse_quote! {fn bar() -> Result<impl Bundle, ()>{} },
		)
		.xpect_eq(syn::parse_quote! {-> Result<impl Bundle + use<>, ()> });
	}


	#[test]
	fn segments() {
		let a: PathSegment = syn::parse_quote! {Foo};
		a.ident.xpect_eq("Foo");
		let a: PathSegment = syn::parse_quote! {Foo<Bar>};
		a.ident.xpect_eq("Foo");
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
		.xpect_snapshot();
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
		.xpect_snapshot();
	}
}

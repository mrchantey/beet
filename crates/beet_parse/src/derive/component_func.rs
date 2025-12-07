use crate::prelude::*;
use beet_core::prelude::*;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use syn::Ident;
use syn::ItemFn;
use syn::Result;
use syn::ReturnType;

pub fn component_func(input: ItemFn, attr: TokenStream) -> TokenStream {
	parse(input, attr).unwrap_or_else(|err| err.into_compile_error())
}

struct Options {
	take: bool,
}
impl Options {
	fn new(attr: TokenStream) -> Result<Self> {
		let attrs = AttributeGroup::parse_punctated(attr)?;
		Ok(Self {
			take: attrs
				.iter()
				.any(|attr| attr.into_token_stream().to_string() == "take"),
		})
	}
}

fn parse(input: ItemFn, attr: TokenStream) -> Result<TokenStream> {
	let opts = Options::new(attr)?;
	let fields = NodeField::parse_item_fn(&input)?;
	let define_struct = define_struct(&input, &opts, &fields)?;
	let impl_on_add = impl_on_add(&input, &opts, &fields)?;

	let imports = if pkg_ext::is_internal() {
		quote! {
			use beet_core::prelude::*;
		}
	} else {
		quote! {
			use beet::prelude::*;
		}
	};

	Ok(quote! {
		#imports
		#define_struct
		#impl_on_add
	})
}

fn on_add_ident(func: &ItemFn) -> Ident {
	use heck::ToSnakeCase;
	let ident_str =
		format!("on_add_{}", func.sig.ident.to_string().to_snake_case());
	Ident::new(&ident_str, func.sig.ident.span())
}

fn define_struct(
	func: &ItemFn,
	opts: &Options,
	fields: &[NodeField],
) -> Result<TokenStream> {
	let attrs = &func.attrs;

	let (_, type_generics, where_clause) = func.sig.generics.split_for_impl();

	let struct_fields = prop_fields(fields).map(|f| {
		let ident = &f.ident;
		let attrs = &f.attrs;
		let ty = f.ty;
		quote! {
			#(#attrs)*
			pub #ident: #ty
		}
	});
	let vis = &func.vis;
	let ident = &func.sig.ident;
	let on_add = on_add_ident(func);

	let mut derives = vec![quote! {Component}];

	if !opts.take {
		derives.push(quote! {Clone});
	};
	if fields.is_empty() {
		derives.push(quote! {Default});
	}

	Ok(quote! {
	#(#attrs)*
	#[derive(#(#derives),*)]
	#[component(on_add = #on_add)]
	#vis struct #ident #type_generics #where_clause {
		#(#struct_fields),*
	}
	})
}

fn impl_on_add(
	func: &ItemFn,
	opts: &Options,
	fields: &[NodeField],
) -> Result<TokenStream> {
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

	let return_type = with_captured_lifetimes(&func);

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

	let maybe_generics = if returns_result {
		quote!(::<_, Result<_>, _, _>)
	} else {
		Default::default()
	};

	let on_add = on_add_ident(func);
	let body = &func.block.stmts;
	let func_body = quote! {
		#[allow(unused_variables, unused_assignments)]
		let #ident { #(#destructure_props),* } = this;
		#(#body)*
	};

	let is_system = fields.iter().any(|f| is_param(f));
	let is_async = func.sig.asyncness.is_some();


	let inner = match (is_async, is_system) {
		(true, true) => {
			quote! {
				compile_error!("async constructors cannot have system params. please use `AsyncEntity`");
			}
		}
		(true, false) => {
			quote! {
				world.run_async_local(async move |world| {
					let #entity_ident = world.entity(id);
					let bundle = {
						#func_body
					 };
					#entity_ident.insert(bundle).await;
				});
			}
		}
		(false, true) => {
			// the constructor is a system
			quote! {
				let bundle = {
					fn system(#[allow(unused_variables, unused_assignments)]In((#entity_ident, this)): In<(Entity, #ident)>, #(#param_fields),*) #return_type {
						#func_body
					}

					entity_world_mut.world_scope(|world| {
						world.run_system_cached_with #maybe_generics(system, (id, this)).map_err(|err|
							bevyhow!(#err_msg, err)
						).unwrap_or_exit()
					})#maybe_unwrap
				};
				entity_world_mut.insert(bundle);
			}
		}
		(false, false) => {
			// the constructor simply accepts its component
			quote! {
				let bundle = {
					#[allow(unused_variables, unused_assignments)]
					let #entity_ident = id;
					#func_body
				};
				entity_world_mut.insert(bundle);
			}
		}
	};

	let this = if opts.take {
		quote! { entity_world_mut.take::<#ident>().unwrap(); }
	} else {
		quote! { entity_world_mut.get::<#ident>().unwrap().clone(); }
	};

	Ok(quote! {
		fn #on_add(mut world: DeferredWorld, cx: HookContext) {
			// let component = world.entity(cx.entity).get::<Foo>().unwrap();
			let entity = cx.entity;
			world.commands().queue(move |world: &mut World| {
				let mut entity_world_mut = world.entity_mut(entity);
				let id = entity_world_mut.id();
				let this = #this
				#inner
			});
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


/// Gets all non system param fields
fn prop_fields<'a>(
	fields: &'a [NodeField],
) -> impl Iterator<Item = &'a NodeField<'a>> {
	fields
		.iter()
		.filter(|f| !is_entity_param(f))
		.filter(|f| !is_param(f))
}

fn system_param_fields<'a>(
	fields: &'a [NodeField],
) -> impl Iterator<Item = &'a NodeField<'a>> {
	fields
		.iter()
		.filter(|f| !is_entity_param(f))
		.filter(|f| is_param(f))
}

fn is_param(field: &NodeField) -> bool {
	const SYSTEM_PARAM_IDENTS: [&str; 7] = [
		"World",
		"Commands",
		"Res",
		"ResMut",
		"Query",
		"Populated",
		"When",
	];
	SYSTEM_PARAM_IDENTS
		.iter()
		.any(|id| field.last_segment_matches(id))
		|| field.field_attributes.contains("param")
}


fn is_entity_param(field: &NodeField) -> bool {
	field.last_segment_matches("Entity")
		|| field.last_segment_matches("AsyncEntity")
}

fn entity_param_ident<'a>(fields: &'a [NodeField]) -> Option<&'a Ident> {
	fields
		.iter()
		.find(|field| is_entity_param(field))
		.map(|field| field.ident)
}


#[cfg(test)]
mod test {
	use super::with_captured_lifetimes;
	use crate::prelude::*;
	use beet_core::prelude::*;
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
		component_func(
			syn::parse_quote! {
				/// probably the best templating layout ever
				pub(crate) fn MyNode(
					/// some comment
					foo:u32,
					mut bar:u32
				) -> impl Bundle{()}
			},
			default(),
		)
		.xpect_snapshot();
	}
	#[test]
	fn take() {
		component_func(
			syn::parse_quote! {
				/// probably the best templating layout ever
				pub(crate) fn MyNode(
					/// some comment
					foo:u32,
					mut bar:u32
				) -> impl Bundle{()}
			},
			quote::quote! {take},
		)
		.xpect_snapshot();
	}
	#[test]
	fn system() {
		component_func(
			syn::parse_quote! {
				/// probably the best templating layout ever
				pub(crate) fn MyNode(
					/// some comment
					foo:u32,
					mut my_res: Res<Time>
				) -> impl Bundle{()}
			},
			default(),
		)
		.xpect_snapshot();
	}
	#[test]
	fn test_async() {
		component_func(
			syn::parse_quote! {
				/// probably the best templating layout ever
				pub(crate) async fn MyNode(
					/// some comment
					foo: u32,
					bar: AsyncEntity,
				) -> impl Bundle{()}
			},
			default(),
		)
		.xpect_snapshot();
	}
	#[test]
	fn complex() {
		component_func(
			syn::parse_quote! {
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
			},
			default(),
		)
		.xpect_snapshot();
	}
}

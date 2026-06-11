//! Implementation of the `#[template]` attribute macro.
//!
//! Turns a function component `fn Name(p1: T1, p2: T2, ..) -> impl Bundle`
//! (body written as `rsx! { .. }`) into a registered template on the substrate:
//!
//! - a data struct `Name { p1, p2, .. }` deriving `Default` + `Clone` +
//!   `Reflect` (`#[reflect(Default)]`), whose fields are the props;
//! - a build-subtree `impl Template<Output = ()>` for `Name` that checks
//!   required props, binds the props by name, runs the body verbatim (any
//!   `rsx!` inside expands normally), and inserts the resulting bundle into the
//!   build target;
//! - a `subtree_template!(Name)` opt-out of Bevy's blanket `Template` impl;
//! - a `register_Name` registration fn (called by `register_template::<Name>()`)
//!   exposing the template by name to the loader.
//!
//! There is no `NameProps` struct and no marker component: the data struct *is*
//! the props, and props are runtime-verified input values, not a compile-time
//! call-site contract. Capitalized tags in `rsx!` lower to
//! `Name { p1: x.into(), ..Default::default() }.into_snippet_bundle()`,
//! dispatched to build at runtime.
//!
//! # Prop grammar
//!
//! - bare field / `#[prop(default)]` -> optional, `Default::default()`
//! - `#[prop(default = expr)]` -> optional, defaults to `expr`
//! - `Option<T>` field -> optional, stored as `PropOpt<T>`, defaults to `None`,
//!   bound back to `Option<T>` in the body
//! - `#[prop(required)]` -> required; stored as `PropOpt<T>`, validated at build
//!   time, surfacing [`MissingProps`] through the build channel (never a panic)
//! - `#[prop(into)]` -> binds the concrete type (the `rsx!` call site already
//!   `.into()`s every value)
//!
//! `#[prop(all)]` is removed entirely.
extern crate alloc;
use alloc::string::ToString;
use alloc::vec::Vec;
use beet_core_shared::prelude::*;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::FnArg;
use syn::ItemFn;
use syn::parse_macro_input;

pub fn impl_template(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	parse(attr.into(), parse_macro_input!(item as ItemFn))
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn parse(attr: TokenStream, item: ItemFn) -> syn::Result<TokenStream> {
	let attrs = AttributeMap::parse(attr)?;
	attrs.assert_types(&[], &["system"])?;

	if attrs.contains_key("system") {
		parse_system(item)
	} else {
		parse_pure(item)
	}
}

/// A `#[template]` parameter lowered to a data-struct field.
struct Prop {
	/// parameter name (also the field name)
	ident: syn::Ident,
	/// type as written by the author — what the body binds
	ty: syn::Type,
	/// stored as `PropOpt<inner>` and validated at build time
	required: bool,
	/// the `T` of a declared `Option<T>` prop, stored as `PropOpt<T>`
	option_inner: Option<syn::Type>,
	/// `#[prop(default = expr)]` default expression
	default_expr: Option<syn::Expr>,
	/// non-`#[prop]` attributes (doc comments etc) kept on the field
	other_attrs: Vec<syn::Attribute>,
}

impl Prop {
	/// Whether the prop is stored as a `PropOpt<_>` (a required prop, or a
	/// declared `Option<T>` prop).
	fn is_opt(&self) -> bool {
		self.required || self.option_inner.is_some()
	}

	/// The field's stored type. A required prop or a declared `Option<T>` prop
	/// stores `PropOpt<inner>` (so the call-site conversion stays unambiguous);
	/// everything else stores its declared type.
	fn stored_ty(&self, beet_core: &syn::Path) -> TokenStream {
		if self.required {
			let ty = &self.ty;
			quote! { #beet_core::prelude::PropOpt<#ty> }
		} else if let Some(inner) = &self.option_inner {
			quote! { #beet_core::prelude::PropOpt<#inner> }
		} else {
			let ty = &self.ty;
			quote! { #ty }
		}
	}

	/// The struct field definition with forwarded attrs.
	///
	/// Fields are `pub` so a `<Name field=x/>` struct-literal patch resolves
	/// across module boundaries (the same crate, a different module).
	fn field_def(&self, beet_core: &syn::Path) -> TokenStream {
		let ident = &self.ident;
		let stored_ty = self.stored_ty(beet_core);
		let other_attrs = &self.other_attrs;
		quote! {
			#(#other_attrs)*
			pub #ident: #stored_ty
		}
	}

	/// The field's value inside a manual `Default` impl. Only a non-`PropOpt`
	/// field carries a `default_expr`; every other field defaults by type
	/// (`PropOpt` to `None`).
	fn default_value(&self) -> TokenStream {
		match (self.is_opt(), &self.default_expr) {
			(false, Some(expr)) => quote! { (#expr).into() },
			_ => quote! { ::core::default::Default::default() },
		}
	}

	/// The body binding that rebinds the stored field to the declared type:
	/// a `PropOpt`-stored optional prop unwraps to `Option<T>`. A required prop
	/// is unwrapped separately (after the missing check).
	fn body_binding(&self) -> Option<TokenStream> {
		if self.option_inner.is_some() && !self.required {
			let ident = &self.ident;
			Some(quote! { let #ident = #ident.into_inner(); })
		} else {
			None
		}
	}
}

/// Parse one parameter into a [`Prop`].
fn parse_prop(pt: &syn::PatType) -> syn::Result<Prop> {
	let ident = param_ident(pt)?;
	let ty = (*pt.ty).clone();

	let mut required = false;
	let mut default_expr = None;
	let mut other_attrs: Vec<syn::Attribute> = Vec::new();

	for attr in &pt.attrs {
		if !attr.path().is_ident("prop") {
			other_attrs.push(attr.clone());
			continue;
		}
		let tokens = match &attr.meta {
			syn::Meta::List(list) => list.tokens.clone(),
			syn::Meta::Path(_) => TokenStream::new(), // bare `#[prop]`
			syn::Meta::NameValue(_) => {
				synbail!(attr, "`#[prop = ..]` form is not supported, use #[prop(..)]")
			}
		};
		let map = AttributeMap::parse(tokens)?;
		for key in map.keys() {
			match key {
				"required" => required = true,
				"default" => default_expr = map.get("default").cloned(),
				// `into` is call-site sugar; the body binds the concrete type.
				"into" => {}
				"all" => synbail!(
					attr,
					"`#[prop(all)]` has been removed; declare each prop"
				),
				other => synbail!(
					attr,
					"unknown `#[prop({other})]` argument"
				),
			}
		}
	}

	// a declared `Option<T>` prop stores `PropOpt<T>` and binds `Option<T>`.
	let option_inner = (!required).then(|| option_inner_type(&ty)).flatten();

	Ok(Prop {
		ident,
		ty,
		required,
		option_inner,
		default_expr,
		other_attrs,
	})
}

/// The `T` of an `Option<T>` type, or `None` if `ty` is not an `Option`.
fn option_inner_type(ty: &syn::Type) -> Option<syn::Type> {
	let syn::Type::Path(tp) = ty else {
		return None;
	};
	let seg = tp.path.segments.last()?;
	if seg.ident != "Option" {
		return None;
	}
	let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
		return None;
	};
	args.args.iter().find_map(|arg| match arg {
		syn::GenericArgument::Type(inner) => Some(inner.clone()),
		_ => None,
	})
}

/// Whether a parameter is a prop (carries a `#[prop]` attribute).
fn is_prop_param(pt: &syn::PatType) -> bool {
	pt.attrs.iter().any(|attr| attr.path().is_ident("prop"))
}

/// Extract the identifier from a simple parameter pattern.
fn param_ident(pt: &syn::PatType) -> syn::Result<syn::Ident> {
	match pt.pat.as_ref() {
		syn::Pat::Ident(pi) => Ok(pi.ident.clone()),
		other => {
			synbail!(other, "`#[template]` parameters must be plain identifiers")
		}
	}
}

/// Coerce a typed function argument, rejecting `self`.
fn typed_arg(arg: &FnArg) -> syn::Result<&syn::PatType> {
	match arg {
		FnArg::Typed(pt) => Ok(pt),
		FnArg::Receiver(recv) => {
			synbail!(recv, "`#[template]` functions cannot take `self`")
		}
	}
}

/// Build the data struct + `Template` impl + registration for a pure template.
fn parse_pure(item: ItemFn) -> syn::Result<TokenStream> {
	let mut props: Vec<Prop> = Vec::new();
	for arg in &item.sig.inputs {
		props.push(parse_prop(typed_arg(arg)?)?);
	}
	emit(&item, &props, /* system */ None)
}

/// Build the data struct + `Template` impl for a `#[template(system)]`.
///
/// `#[prop]` params are fields; every other param is a Bevy `SystemParam`
/// fetched synchronously at build time via [`system_template`].
fn parse_system(item: ItemFn) -> syn::Result<TokenStream> {
	let mut props: Vec<Prop> = Vec::new();
	let mut sys_types: Vec<TokenStream> = Vec::new();
	let mut sys_pats: Vec<syn::Ident> = Vec::new();
	for arg in &item.sig.inputs {
		let pt = typed_arg(arg)?;
		if is_prop_param(pt) {
			props.push(parse_prop(pt)?);
		} else {
			let ty = &pt.ty;
			sys_types.push(quote! { #ty });
			sys_pats.push(param_ident(pt)?);
		}
	}
	emit(&item, &props, Some(System { sys_types, sys_pats }))
}

/// The system-template data for a `#[template(system)]`.
struct System {
	sys_types: Vec<TokenStream>,
	sys_pats: Vec<syn::Ident>,
}

/// Emit the data struct, `Template` impl, `subtree_template!`, and registration
/// shared by the pure and system paths.
fn emit(
	item: &ItemFn,
	props: &[Prop],
	system: Option<System>,
) -> syn::Result<TokenStream> {
	let vis = &item.vis;
	let name = &item.sig.ident;
	let fn_attrs = &item.attrs;
	let body = &item.block;

	let beet_core = pkg_ext::internal_or_beet("beet_core");
	let bevy = pkg_ext::bevy();

	// thread any generics through every impl; empty generics emit nothing, so the
	// non-generic path is unchanged. Trait bounds the generated impls need are
	// deferred onto `Self`, so the author only declares their own param bounds.
	let generics = &item.sig.generics;
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
	// `Self`-deferred bounds the generated impls require, no-ops when non-generic.
	let unpin_where = merge_where(
		generics,
		quote! {
			for<'a> [()]:
				#beet_core::exports::bevy::ecs::template::SpecializeFromTemplate
		},
	);
	let template_where =
		merge_where(generics, quote! { Self: ::core::clone::Clone });
	let build_template_where = merge_where(
		generics,
		quote! {
			Self: 'static
				+ ::core::marker::Send
				+ ::core::marker::Sync
				+ ::core::clone::Clone
				+ #bevy::ecs::template::Template<Output = ()>
		},
	);
	let schema_where =
		merge_where(generics, quote! { Self: #bevy::reflect::Typed });
	let register_where = merge_where(
		generics,
		quote! {
			Self: #bevy::reflect::FromReflect
				+ #bevy::reflect::Typed
				+ #bevy::reflect::GetTypeRegistration
				+ #beet_core::prelude::GetTemplateSchema
				+ #bevy::ecs::template::Template<Output = ()>
		},
	);

	let field_idents: Vec<&syn::Ident> =
		props.iter().map(|prop| &prop.ident).collect();
	// the names of required props, so the schema can mark them required.
	let required_names: Vec<syn::LitStr> = props
		.iter()
		.filter(|prop| prop.required)
		.map(|prop| {
			syn::LitStr::new(&prop.ident.to_string(), prop.ident.span())
		})
		.collect();
	let data_struct = data_struct(vis, name, generics, props, &beet_core);

	let required_checks = required_checks(props, &beet_core);
	let required_unwraps = required_unwraps(props);
	// rebind `PropOpt`-stored optional props to `Option<T>` for the body.
	let body_bindings: Vec<TokenStream> =
		props.iter().filter_map(Prop::body_binding).collect();

	let is_system = system.is_some();
	// the inner build: bind props, run the body verbatim (it ends in an
	// `impl Bundle`, commonly an `rsx!` that expands through the normal macro
	// path), insert the resulting bundle into the build target.
	let build_body = match system {
		Some(System { sys_types, sys_pats }) => quote! {
			let inner = #beet_core::prelude::system_template::<
				(#(#sys_types,)*), _, _
			>(move |_entity, (#(#sys_pats,)*)| {
				let Self { #(#field_idents),* } = props.clone();
				#(#required_unwraps)*
				#(#body_bindings)*
				let bundle = { use #beet_core::prelude::*; #body };
				#beet_core::prelude::Snippet::from_bundle(bundle)
			});
			cx.entity.build_template(&inner)
		},
		None => quote! {
			let Self { #(#field_idents),* } = self.clone();
			#(#required_unwraps)*
			#(#body_bindings)*
			let bundle = { use #beet_core::prelude::*; #body };
			cx.entity.insert(bundle);
			::core::result::Result::Ok(())
		},
	};

	// system templates move `self` into a `FnOnce`, so bind a clone first.
	let props_binding =
		is_system.then(|| quote! { let props = self.clone(); });

	let register_fn = format_ident!("register_{}", name);

	// a `#[prop(default = expr)]` forces a manual `Default` impl (emitted by
	// `data_struct`), so omit the `Default` derive to avoid a conflict.
	let needs_manual_default =
		props.iter().any(|prop| prop.default_expr.is_some());
	let default_derive = (!needs_manual_default).then(|| quote! { Default, });

	Ok(quote! {
		#(#fn_attrs)*
		#[derive(
			#default_derive
			Clone,
			#bevy::reflect::Reflect,
		)]
		#[reflect(Default)]
		#[allow(non_snake_case)]
		#data_struct

		// opt out of Bevy's blanket `Template for T: Default + Clone + Unpin`,
		// generics-aware (the `subtree_template!` macro cannot express generics).
		impl #impl_generics ::core::marker::Unpin for #name #ty_generics #unpin_where
		{
		}

		impl #impl_generics #bevy::ecs::template::Template for #name #ty_generics #template_where {
			type Output = ();
			#[track_caller]
			#[allow(non_snake_case, unused_variables, unused_braces, unused_imports)]
			fn build_template(
				&self,
				cx: &mut #bevy::ecs::template::TemplateContext,
			) -> #bevy::ecs::error::Result<()> {
				let location = ::core::panic::Location::caller();
				let mut missing = #beet_core::prelude::Vec::new();
				#(#required_checks)*
				if !missing.is_empty() {
					return ::core::result::Result::Err(
						#beet_core::prelude::MissingProps {
							props: missing,
							location,
						}.into(),
					);
				}
				#props_binding
				#build_body
			}
			fn clone_template(&self) -> Self { self.clone() }
		}

		// marks this as a build-subtree template, so `<#name .../>` dispatches to
		// build rather than insert (distinguishing it from a reflect-patch
		// component, which Bevy's blanket `Template` impl would otherwise shadow).
		impl #impl_generics #beet_core::prelude::BuildTemplate for #name #ty_generics #build_template_where {}

		// the prop schema, authored by the typed signature: starts from the
		// reflect-derived struct schema (a `PropOpt<T>` prop is an optional inner
		// schema), then marks `#[prop(required)]` props as required, which the type
		// alone cannot express. The loader verifies a prop set against this.
		impl #impl_generics #beet_core::prelude::GetTemplateSchema for #name #ty_generics #schema_where {
			fn template_schema() -> #beet_core::prelude::ValueSchema {
				let mut schema = #beet_core::prelude::ValueSchema::of::<Self>();
				let required: &[&str] = &[#(#required_names),*];
				if let #beet_core::prelude::ValueSchema::Struct(ref mut struct_schema) = schema {
					for field in struct_schema.fields.iter_mut() {
						field.required = required.contains(&field.key.as_str());
					}
				}
				schema
			}
		}

		impl #impl_generics #name #ty_generics #where_clause {
			/// Registers this template by name on the world's type registry,
			/// attaching its prop schema alongside the build bridge.
			#[allow(dead_code, non_snake_case)]
			#vis fn #register_fn(
				world: &mut #bevy::ecs::world::World,
			) #register_where {
				#beet_core::prelude::WorldRegisterTemplateExt::register_template::<Self>(world);
			}
		}
	})
}

/// Build a `where` clause merging the author's predicates (if any) with `extra`
/// bounds the generated impl needs, deferred onto `Self` so a generic template
/// only declares its own param bounds.
fn merge_where(generics: &syn::Generics, extra: TokenStream) -> TokenStream {
	match &generics.where_clause {
		Some(clause) => {
			let predicates = &clause.predicates;
			quote! { where #predicates, #extra }
		}
		None => quote! { where #extra },
	}
}

/// The data struct definition. Derives `Default` directly unless a
/// `#[prop(default = expr)]` forces a manual `Default` impl.
fn data_struct(
	vis: &syn::Visibility,
	name: &syn::Ident,
	generics: &syn::Generics,
	props: &[Prop],
	beet_core: &syn::Path,
) -> TokenStream {
	let field_defs: Vec<TokenStream> =
		props.iter().map(|prop| prop.field_def(beet_core)).collect();
	let needs_manual_default =
		props.iter().any(|prop| prop.default_expr.is_some());

	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
	let struct_def = if field_defs.is_empty() {
		quote! { #vis struct #name #generics #where_clause; }
	} else {
		quote! { #vis struct #name #generics #where_clause { #(#field_defs),* } }
	};

	if needs_manual_default {
		let field_idents = props.iter().map(|prop| &prop.ident);
		let defaults = props.iter().map(Prop::default_value);
		quote! {
			#struct_def

			impl #impl_generics ::core::default::Default for #name #ty_generics #where_clause {
				fn default() -> Self {
					Self {
						#(#field_idents: #defaults),*
					}
				}
			}
		}
	} else {
		struct_def
	}
}

/// `if <field>.is_none() { missing.push("<field>"); }` for each required prop.
fn required_checks(
	props: &[Prop],
	beet_core: &syn::Path,
) -> Vec<TokenStream> {
	props
		.iter()
		.filter(|prop| prop.required)
		.map(|prop| {
			let ident = &prop.ident;
			let lit = syn::LitStr::new(&ident.to_string(), ident.span());
			quote! {
				if self.#ident.is_none() {
					missing.push(
						#beet_core::prelude::SmolStr::new_static(#lit)
					);
				}
			}
		})
		.collect()
}

/// `let <field> = <field>.into_inner().unwrap();` for each required prop —
/// after validation each binding matches its originally declared type. The
/// stored field is a `PropOpt<T>`, so unwrap its inner `Option`.
fn required_unwraps(props: &[Prop]) -> Vec<TokenStream> {
	props
		.iter()
		.filter(|prop| prop.required)
		.map(|prop| {
			let ident = &prop.ident;
			quote! { let #ident = #ident.into_inner().unwrap(); }
		})
		.collect()
}

#[cfg(test)]
mod test {
	use super::*;
	use alloc::string::String;
	use alloc::string::ToString;
	use quote::quote;

	fn parse_str(attr: TokenStream, item: syn::ItemFn) -> String {
		parse(attr, item).unwrap().to_string()
	}

	fn parse_err(attr: TokenStream, item: syn::ItemFn) -> String {
		parse(attr, item).unwrap_err().to_string()
	}

	#[test]
	fn generates_data_struct_and_template() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Button(label: String, variant: u32) -> impl Bundle {
				rsx! { <button>{label}</button> }
			}
		});
		// data struct carrying the props, no `*Props`
		assert!(result.contains("struct Button"));
		assert!(result.contains("label : String"));
		assert!(!result.contains("ButtonProps"));
		// reflect + default for the loader
		assert!(result.contains("Reflect"));
		assert!(result.contains("reflect (Default)"));
		// the subtree-template opt-out (inlined `Unpin`) + Template impl
		assert!(result.contains("Unpin for Button"));
		assert!(result.contains("Template for Button"));
		assert!(result.contains("let Self { label , variant } = self . clone ()"));
		// registration fn
		assert!(result.contains("fn register_Button"));
		// the prop schema, authored by the typed signature
		assert!(result.contains("GetTemplateSchema for Button"));
		assert!(result.contains("fn template_schema"));
		// no scene machinery
		assert!(!result.contains("SceneComponent"));
	}

	#[test]
	fn required_prop_generates_checked_path() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Field(#[prop(required)] variant: Variant) -> impl Bundle {
				rsx! { <span/> }
			}
		});
		// stored as PropOpt, validated, unwrapped
		assert!(result.contains("PropOpt < Variant >"));
		assert!(result.contains("if self . variant . is_none ()"));
		assert!(result.contains("MissingProps"));
		assert!(result.contains("let variant = variant . into_inner () . unwrap ()"));
	}

	#[test]
	fn default_expr_generates_manual_default() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Field(#[prop(default = "hi")] placeholder: String) -> impl Bundle {
				rsx! { <span/> }
			}
		});
		assert!(result.contains("impl :: core :: default :: Default for Field"));
		assert!(result.contains("\"hi\""));
	}

	#[test]
	fn no_props_unit_struct() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Spacer() -> impl Bundle { rsx! { <br/> } }
		});
		assert!(result.contains("struct Spacer ;"));
		assert!(result.contains("Template for Spacer"));
	}

	#[test]
	fn system_props_and_params() {
		let result = parse_str(quote!(system), syn::parse_quote! {
			fn Panel(#[prop] role: ColorRole, theme: Res<Theme>) -> impl Bundle {
				rsx! { <div/> }
			}
		});
		assert!(result.contains("struct Panel"));
		assert!(result.contains("role : ColorRole"));
		assert!(result.contains("system_template :: < (Res < Theme > ,)"));
		assert!(result.contains("let Self { role } = props . clone ()"));
	}

	#[test]
	fn prop_all_rejected() {
		let err = parse_err(quote!(), syn::parse_quote! {
			fn Field(#[prop(all)] cfg: FieldProps) -> impl Bundle {
				rsx! { <span/> }
			}
		});
		assert!(err.contains("removed"));
	}

	#[test]
	fn rejects_self() {
		let err = parse_err(quote!(), syn::parse_quote! {
			fn Bad(self) -> impl Bundle { rsx! { <span/> } }
		});
		assert!(err.contains("self"));
	}

	#[test]
	fn supports_generics() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Wrapper<T: Bundle>(inner: T) -> impl Bundle {
				rsx! { <div>{inner}</div> }
			}
		});
		// generics thread through the struct and every impl.
		assert!(result.contains("struct Wrapper < T : Bundle >"));
		assert!(result.contains("Template for Wrapper < T >"));
		assert!(result.contains("inner : T"));
	}
}

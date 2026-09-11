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
//! call-site contract. Capitalized tags in `rsx!` lower to assignments over
//! `Name::default()` (`props.p1 = x.into_prop()`), dispatched to build at
//! runtime.
//!
//! # Prop grammar
//!
//! - bare field / `#[prop(default)]` -> optional, `Default::default()`
//! - `#[prop(default = expr)]` -> optional, defaults to `expr`
//! - `Option<T>` field -> optional, defaults to `None`
//! - `#[prop(required)]` -> required; stored as `Option<T>`, validated at build
//!   time, surfacing [`MissingProps`] through the build channel (never a panic)
//!
//! The grammar itself lives in
//! [`beet_core_shared::prelude::Prop`], shared with the `#[action]` macro's
//! `#[field]` spelling so the two cannot drift.
extern crate alloc;
use alloc::string::ToString;
use alloc::vec::Vec;
use beet_core_shared::prelude::*;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::ItemFn;
use syn::parse_macro_input;

pub(crate) fn impl_template(
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

/// Whether a system-template param is the building [`Entity`] rather than a
/// [`SystemParam`], ie a bare `entity: Entity`.
fn is_entity_param(pt: &syn::PatType) -> bool {
	let syn::Type::Path(path) = pt.ty.as_ref() else {
		return false;
	};
	path.path
		.segments
		.last()
		.is_some_and(|segment| segment.ident == "Entity")
}

/// Build the data struct + `Template` impl + registration for a pure template.
fn parse_pure(item: ItemFn) -> syn::Result<TokenStream> {
	let mut props: Vec<Prop> = Vec::new();
	for arg in &item.sig.inputs {
		props.push(Prop::parse(Prop::typed_arg(arg, "template")?, "prop")?);
	}
	emit(&item, &props, /* system */ None)
}

/// Build the data struct + `Template` impl for a `#[template(system)]`.
///
/// `#[prop]` params are fields; every other param is a Bevy `SystemParam`
/// fetched synchronously at build time via [`SystemTemplate`].
fn parse_system(item: ItemFn) -> syn::Result<TokenStream> {
	let mut props: Vec<Prop> = Vec::new();
	let mut sys_types: Vec<TokenStream> = Vec::new();
	let mut sys_pats: Vec<syn::PatIdent> = Vec::new();
	let mut entity_pat: Option<syn::PatIdent> = None;
	for arg in &item.sig.inputs {
		let pt = Prop::typed_arg(arg, "template")?;
		if Prop::is_param(pt, "prop") {
			props.push(Prop::parse(pt, "prop")?);
		} else if is_entity_param(pt) {
			// the entity being built, so the body can read self/ancestor context
			// (`<LightsailBeetSiteBlock/>` resolving its deploy scope by ancestry).
			entity_pat = Some(Prop::param_pat_ident(pt, "template")?);
		} else {
			let ty = &pt.ty;
			sys_types.push(quote! { #ty });
			// keep the full `PatIdent` so a `mut` binding (eg `mut meshes:
			// ResMut<..>`) stays mutable in the build closure pattern.
			sys_pats.push(Prop::param_pat_ident(pt, "template")?);
		}
	}
	emit(
		&item,
		&props,
		Some(System {
			sys_types,
			sys_pats,
			entity_pat,
		}),
	)
}

/// Whether the template returns a `Result<impl Bundle>`, so the body's value is
/// unwrapped with `?` into the enclosing result: `build_template`'s for a pure
/// template, the [`SystemTemplate`] build closure's for a `#[template(system)]`.
/// Lets a template that can genuinely fail (parsing a declared uri, rendering a
/// deploy config) surface the error where the build already handles one, rather
/// than unwrapping.
fn is_fallible(item: &ItemFn) -> bool {
	let syn::ReturnType::Type(_, ty) = &item.sig.output else {
		return false;
	};
	let syn::Type::Path(path) = ty.as_ref() else {
		return false;
	};
	path.path
		.segments
		.last()
		.is_some_and(|segment| segment.ident == "Result")
}

/// The system-template data for a `#[template(system)]`.
struct System {
	sys_types: Vec<TokenStream>,
	sys_pats: Vec<syn::PatIdent>,
	/// The binding for the entity being built, when the body declared an
	/// `Entity` param.
	entity_pat: Option<syn::PatIdent>,
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

	// `mut` is an action-only key: a template prop is cloned into the body.
	if let Some(prop) = props.iter().find(|prop| prop.mutable) {
		synbail!(
			&prop.ident,
			"`#[prop(mut)]` is not supported, a template prop binds by value"
		);
	}

	let beet_core = pkg_ext::internal_or_beet("beet_core");
	let bevy = pkg_ext::bevy();

	// thread any generics through every impl; empty generics emit nothing, so the
	// non-generic path is unchanged. Trait bounds the generated impls need are
	// deferred onto `Self`, so the author only declares their own param bounds.
	let generics = &item.sig.generics;
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
	// `Self`-deferred bounds the generated impls require, no-ops when non-generic.
	let unpin_where = merge_where(generics, quote! {
		for<'a> [()]:
			#beet_core::exports::bevy::ecs::template::SpecializeFromTemplate
	});
	let template_where =
		merge_where(generics, quote! { Self: ::core::clone::Clone });
	let build_template_where = merge_where(generics, quote! {
		Self: 'static
			+ ::core::marker::Send
			+ ::core::marker::Sync
			+ ::core::clone::Clone
			+ #bevy::ecs::template::Template<Output = ()>
	});
	let schema_where =
		merge_where(generics, quote! { Self: #bevy::reflect::Typed });
	let register_where = merge_where(generics, quote! {
		Self: #bevy::reflect::FromReflect
			+ #bevy::reflect::Typed
			+ #bevy::reflect::GetTypeRegistration
			+ #beet_core::prelude::GetTemplateSchema
			+ #bevy::ecs::template::Template<Output = ()>
	});

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
	let data_struct = data_struct(vis, name, generics, props);

	let required_checks = required_checks(props, &beet_core);
	let required_unwraps = required_unwraps(props);

	let is_system = system.is_some();
	// a `-> Result<impl Bundle>` body unwraps with `?` into the enclosing result
	// (`build_template`'s own, or the build closure's for a system template), so a
	// fallible template needs no unwrap at the author site. The `impl Bundle`
	// return type erases here, so the intermediate is annotated to pin the error
	// type to `BevyError` (the one a template body raises, and the one both
	// enclosing results carry).
	let bundle = match is_fallible(item) {
		true => quote! {
			let bundle: #bevy::ecs::error::Result<_> = #body;
			let bundle = bundle?;
		},
		false => quote! {
			let bundle = #body;
		},
	};
	// the inner build: bind props, run the body verbatim (it ends in an
	// `impl Bundle`, commonly an `rsx!` that expands through the normal macro
	// path), insert the resulting bundle into the build target.
	let build_body = match system {
		Some(System {
			sys_types,
			sys_pats,
			entity_pat,
		}) => {
			let entity_binding = entity_pat
				.clone()
				.map(|pat| quote! { #pat })
				.unwrap_or_else(|| quote! { _entity });
			quote! {
				let inner = #beet_core::prelude::SystemTemplate::<
					(#(#sys_types,)*), _, _
				>::new(move |#entity_binding, (#(#sys_pats,)*)| {
					let Self { #(#field_idents),* } = props.clone();
					#(#required_unwraps)*
					#bundle
					::core::result::Result::Ok(
						#beet_core::prelude::Snippet::from_bundle(bundle)
					)
				});
				cx.entity.build_template(&inner)
			}
		}
		None => quote! {
			let Self { #(#field_idents),* } = self.clone();
			#(#required_unwraps)*
			#bundle
			cx.entity.insert(bundle);
			::core::result::Result::Ok(())
		},
	};

	// system templates move `self` into a `FnOnce`, so bind a clone first.
	let props_binding = is_system.then(|| quote! { let props = self.clone(); });

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
		// reflect-derived struct schema (an `Option<T>` prop is an optional inner
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
) -> TokenStream {
	let field_defs: Vec<TokenStream> =
		props.iter().map(Prop::field_def).collect();
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
fn required_checks(props: &[Prop], beet_core: &syn::Path) -> Vec<TokenStream> {
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

/// `let <field> = <field>.unwrap();` for each required prop: after validation
/// each binding matches its originally declared type.
fn required_unwraps(props: &[Prop]) -> Vec<TokenStream> {
	props
		.iter()
		.filter(|prop| prop.required)
		.map(|prop| {
			let ident = &prop.ident;
			quote! { let #ident = #ident.unwrap(); }
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
		assert!(
			result.contains("let Self { label , variant } = self . clone ()")
		);
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
		// stored as an `Option`, validated, unwrapped
		assert!(result.contains("Option < Variant >"));
		assert!(result.contains("if self . variant . is_none ()"));
		assert!(result.contains("MissingProps"));
		assert!(result.contains("let variant = variant . unwrap ()"));
	}

	#[test]
	fn default_expr_generates_manual_default() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Field(#[prop(default = "hi")] placeholder: String) -> impl Bundle {
				rsx! { <span/> }
			}
		});
		assert!(
			result.contains("impl :: core :: default :: Default for Field")
		);
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
		assert!(result.contains("SystemTemplate :: < (Res < Theme > ,)"));
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

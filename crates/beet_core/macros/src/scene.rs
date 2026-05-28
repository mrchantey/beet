//! Implementation of the `#[scene]` attribute macro.
//!
//! Turns a Leptos/Solid-style function component
//! `fn Name(p1: T1, p2: T2, ..) -> impl Scene` into a Bevy
//! [`SceneComponent`](bevy::scene::SceneComponent) marker:
//! - a unit struct `Name` deriving `SceneComponent` + `Default` + `Clone` +
//!   `Reflect`, with `#[scene(NameProps)]` pointing at the props type;
//! - a props struct `NameProps { p1, p2, .. }` with `Default` + `SetWith`
//!   setters (per-param `#[prop(into)]` becomes `#[set_with(into)]`);
//! - an inherent `impl Name { fn scene(props: NameProps) -> impl Scene }` whose
//!   body is the original body with the props destructured into the named
//!   params.
//!
//! Capitalized tags in `rsx!` lower to
//! `<Name as SceneComponent>::scene(NameProps::default().with_p1(..))`, so
//! omitted attributes fall back to `Default` and the entity gains the `Name`
//! component (unlocking `:Name { … }` inheritance, caching, reflection).
extern crate alloc;
use alloc::vec::Vec;
use beet_core_shared::prelude::*;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::FnArg;
use syn::ItemFn;
use syn::parse_macro_input;

pub fn impl_scene(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	parse(attr.into(), parse_macro_input!(item as ItemFn))
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn parse(attr: TokenStream, item: ItemFn) -> syn::Result<TokenStream> {
	let attrs = AttributeMap::parse(attr)?;
	attrs.assert_types(&[], &["system", "component"])?;

	if attrs.contains_key("component") {
		synbail!(
			&item.sig,
			"`#[scene(component)]` is not yet implemented; use a plain `#[scene]`"
		);
	}
	if !item.sig.generics.params.is_empty() {
		synbail!(&item.sig.generics, "`#[scene]` does not yet support generics");
	}
	if attrs.contains_key("system") {
		parse_system(item)
	} else {
		parse_pure(item)
	}
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
			synbail!(other, "`#[scene]` parameters must be plain identifiers")
		}
	}
}

/// Build the marker struct + `SceneComponent` impl + props struct for a pure
/// scene.
fn parse_pure(item: ItemFn) -> syn::Result<TokenStream> {
	let vis = &item.vis;
	let fn_name = &item.sig.ident;
	let body = &item.block;
	let output = &item.sig.output;
	let fn_attrs = &item.attrs;
	let props_name = format_ident!("{}Props", fn_name);

	// each param becomes a props field; `#[prop(..)]` becomes `#[set_with(..)]`
	let mut field_defs: Vec<TokenStream> = Vec::new();
	let mut field_idents: Vec<syn::Ident> = Vec::new();
	for arg in &item.sig.inputs {
		let pt = match arg {
			FnArg::Typed(pt) => pt,
			FnArg::Receiver(recv) => {
				synbail!(recv, "`#[scene]` functions cannot take `self`")
			}
		};
		let ident = param_ident(pt)?;
		let ty = &pt.ty;
		let set_with_attrs = translate_prop_attrs(&pt.attrs)?;
		field_defs.push(quote! { #(#set_with_attrs)* #ident: #ty });
		field_idents.push(ident);
	}

	let beet_core = pkg_ext::internal_or_beet("beet_core");

	let scene_component_impl =
		scene_component_impl(fn_name, &props_name, quote! { Self::scene(props) });

	Ok(quote! {
		#(#fn_attrs)*
		#[derive(
			::bevy::ecs::component::Component,
			Default,
			Clone,
			::bevy::reflect::Reflect,
		)]
		#[reflect(Component)]
		#vis struct #fn_name;

		#[derive(Default, #beet_core::prelude::SetWith)]
		#[allow(non_camel_case_types)]
		#vis struct #props_name {
			#(#field_defs),*
		}

		impl #fn_name {
			#[allow(non_snake_case, unused_variables)]
			#vis fn scene(props: #props_name) #output {
				let #props_name { #(#field_idents),* } = props;
				#[allow(unused_braces)]
				#body
			}
		}

		#scene_component_impl
	})
}

/// Emit the `SceneComponent` trait impl that wraps the inherent
/// `Self::scene(props)` body with the (init-template, scene-component-info)
/// tuple — mirroring what `#[derive(SceneComponent)]` would generate.
fn scene_component_impl(
	fn_name: &syn::Ident,
	props_name: &syn::Ident,
	scene_expr: TokenStream,
) -> TokenStream {
	quote! {
		impl ::bevy::scene::SceneComponent for #fn_name {
			type Props = #props_name;
			fn scene(props: Self::Props) -> impl ::bevy::scene::Scene {
				(
					#scene_expr,
					<::bevy::scene::InitTemplate::<
						<Self as ::bevy::ecs::template::FromTemplate>::Template
					> as ::core::default::Default>::default(),
					::bevy::scene::template_value(
						::bevy::scene::SceneComponentInfo::new::<Self>(true),
					),
				)
			}
		}
	}
}

/// Build the props struct and a build-time callable for a `#[scene(system)]`.
///
/// Props are the parameters marked `#[prop]`; all other parameters are Bevy
/// [`SystemParam`]s fetched synchronously at the scene **build** phase. The
/// body reads them and returns a sub-scene, which is applied to the entity.
fn parse_system(item: ItemFn) -> syn::Result<TokenStream> {
	let vis = &item.vis;
	let fn_name = &item.sig.ident;
	let body = &item.block;
	let output = &item.sig.output;
	let fn_attrs = &item.attrs;
	let props_name = format_ident!("{}Props", fn_name);

	let mut prop_field_defs: Vec<TokenStream> = Vec::new();
	let mut prop_idents: Vec<syn::Ident> = Vec::new();
	let mut sys_types: Vec<&syn::Type> = Vec::new();
	let mut sys_pats: Vec<syn::Ident> = Vec::new();
	for arg in &item.sig.inputs {
		let pt = match arg {
			FnArg::Typed(pt) => pt,
			FnArg::Receiver(recv) => {
				synbail!(recv, "`#[scene]` functions cannot take `self`")
			}
		};
		if is_prop_param(pt) {
			let ident = param_ident(pt)?;
			let ty = &pt.ty;
			let set_with_attrs = translate_prop_attrs(&pt.attrs)?;
			prop_field_defs.push(quote! { #(#set_with_attrs)* #ident: #ty });
			prop_idents.push(ident);
		} else {
			sys_types.push(&pt.ty);
			sys_pats.push(param_ident(pt)?);
		}
	}

	let beet_core = pkg_ext::internal_or_beet("beet_core");
	let beet_ui = pkg_ext::internal_or_beet("beet_ui");

	let scene_component_impl =
		scene_component_impl(fn_name, &props_name, quote! { Self::scene(props) });

	Ok(quote! {
		#(#fn_attrs)*
		#[derive(
			::bevy::ecs::component::Component,
			Default,
			Clone,
			::bevy::reflect::Reflect,
		)]
		#[reflect(Component)]
		#vis struct #fn_name;

		#[derive(Default, Clone, #beet_core::prelude::SetWith)]
		#[allow(non_camel_case_types)]
		#vis struct #props_name {
			#(#prop_field_defs),*
		}

		impl #fn_name {
			#[allow(non_snake_case, unused_variables)]
			#vis fn scene(props: #props_name) #output {
				// build-time scene: fetch the system params, run the body, apply
				// the produced sub-scene to the entity (synchronous world access)
				#beet_ui::prelude::scene_system::<(#(#sys_types,)*), _, _>(
					move |(#(#sys_pats,)*)| {
						let #props_name { #(#prop_idents),* } = props.clone();
						#[allow(unused_braces)]
						#body
					},
				)
			}
		}

		#scene_component_impl
	})
}

/// Translate per-param `#[prop(..)]` attributes into `#[set_with(..)]` on the
/// generated props field. Other attributes (e.g. doc comments) pass through.
fn translate_prop_attrs(
	attrs: &[syn::Attribute],
) -> syn::Result<Vec<TokenStream>> {
	let mut out = Vec::new();
	for attr in attrs {
		if attr.path().is_ident("prop") {
			match &attr.meta {
				syn::Meta::List(list) => {
					let tokens = &list.tokens;
					out.push(quote! { #[set_with(#tokens)] });
				}
				// bare `#[prop]` carries no options
				syn::Meta::Path(_) => {}
				syn::Meta::NameValue(_) => {
					synbail!(attr, "`#[prop = ..]` form is not supported")
				}
			}
		} else {
			out.push(quote! { #attr });
		}
	}
	Ok(out)
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
	fn generates_marker_props_and_scene() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Button(label: String, variant: u32) -> impl Scene { todo!() }
		});
		// marker struct + reflect-component
		assert!(result.contains("struct Button"));
		assert!(result.contains("Component"));
		assert!(result.contains("Reflect"));
		// props struct + SetWith
		assert!(result.contains("struct ButtonProps"));
		assert!(result.contains("SetWith"));
		// inherent scene fn
		assert!(result.contains("impl Button"));
		assert!(result.contains("fn scene (props : ButtonProps)"));
		assert!(result.contains("let ButtonProps { label , variant } = props"));
		// SceneComponent trait impl wrapping the inherent body
		assert!(result.contains("SceneComponent for Button"));
		assert!(result.contains("type Props = ButtonProps"));
		assert!(result.contains("SceneComponentInfo"));
	}

	#[test]
	fn prop_into_becomes_set_with_into() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Button(#[prop(into)] label: String) -> impl Scene { todo!() }
		});
		assert!(result.contains("# [set_with (into)]"));
	}

	#[test]
	fn system_no_props_one_param() {
		// app_info shape: no props, one SystemParam, returns a sub-scene
		let result = parse_str(quote!(system), syn::parse_quote! {
			fn AppInfo(config: Res<PackageConfig>) -> impl Scene { todo!() }
		});
		assert!(result.contains("struct AppInfo"));
		assert!(result.contains("struct AppInfoProps"));
		assert!(result.contains("SceneComponent for AppInfo"));
		assert!(result.contains("scene_system :: < (Res < PackageConfig > ,)"));
		assert!(result.contains("move | (config ,) |"));
	}

	#[test]
	fn system_props_and_params() {
		let result = parse_str(quote!(system), syn::parse_quote! {
			fn Panel(#[prop] role: ColorRole, theme: Res<Theme>) -> impl Scene { todo!() }
		});
		// `role` is a prop field; `theme` is a system param
		assert!(result.contains("struct Panel"));
		assert!(result.contains("struct PanelProps"));
		assert!(result.contains("role : ColorRole"));
		assert!(result.contains("scene_system :: < (Res < Theme > ,)"));
		assert!(result.contains("let PanelProps { role } = props . clone ()"));
	}

	#[test]
	fn component_not_yet_supported() {
		let err = parse_err(quote!(component), syn::parse_quote! {
			fn Panel(role: u32) -> impl Scene { todo!() }
		});
		assert!(err.contains("not yet implemented"));
	}

	#[test]
	fn rejects_self() {
		let err = parse_err(quote!(), syn::parse_quote! {
			fn Bad(self) -> impl Scene { todo!() }
		});
		assert!(err.contains("self"));
	}
}

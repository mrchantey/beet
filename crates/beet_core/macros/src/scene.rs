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
//!
//! # Prop grammar
//!
//! Required-ness is **opt-in** (the upstream `Props: Default` bound makes
//! compile-time required props impossible; see `agent/plans/required_props.md`):
//!
//! - bare field / `#[prop(default)]` → optional, `Default::default()`
//! - `#[prop(default = expr)]` → optional, defaults to `expr`
//! - `Option<T>` field → optional, defaults to `None` (setter takes `T`)
//! - `#[prop(required)]` → required; stored as `Option<T>` and validated at
//!   build time, surfacing [`MissingProps`](beet_ui::prelude::MissingProps)
//!   through the build channel (never a panic) when unset
//! - `#[prop(into)]` → `impl Into` setter
//! - `#[prop(all)]` → the param's type *is* the props type (no struct emitted)
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

/// A `#[scene]` parameter lowered to a props field.
struct Prop {
	/// parameter name (also the props field name)
	ident: syn::Ident,
	/// type as written by the author — what the body binds
	ty: syn::Type,
	/// stored as `Option<ty>` and validated at build time
	required: bool,
	/// `#[prop(default = expr)]` default expression
	default_expr: Option<syn::Expr>,
	/// non-`#[prop]` attributes (doc comments etc) kept on the field
	other_attrs: Vec<syn::Attribute>,
	/// `#[set_with(..)]` argument tokens forwarded to the field
	set_with_args: Vec<TokenStream>,
}

impl Prop {
	/// The field's stored type: `Option<ty>` for required props, else `ty`.
	fn stored_ty(&self) -> TokenStream {
		let ty = &self.ty;
		if self.required {
			quote! { ::core::option::Option<#ty> }
		} else {
			quote! { #ty }
		}
	}

	/// The struct field definition, with forwarded attrs + `#[set_with(..)]`.
	fn field_def(&self) -> TokenStream {
		let ident = &self.ident;
		let stored_ty = self.stored_ty();
		let other_attrs = &self.other_attrs;
		let set_with = (!self.set_with_args.is_empty()).then(|| {
			let args = &self.set_with_args;
			quote! { #[set_with(#(#args),*)] }
		});
		quote! {
			#(#other_attrs)*
			#set_with
			#ident: #stored_ty
		}
	}

	/// The field's value inside a manual `Default` impl.
	fn default_value(&self) -> TokenStream {
		if self.required {
			quote! { ::core::option::Option::None }
		} else if let Some(expr) = &self.default_expr {
			quote! { (#expr).into() }
		} else {
			quote! { ::core::default::Default::default() }
		}
	}
}

/// Parse one parameter into a [`Prop`], reporting whether it carried
/// `#[prop(all)]` (the param's type *is* the props type).
fn parse_prop(pt: &syn::PatType) -> syn::Result<(Prop, bool)> {
	let ident = param_ident(pt)?;
	let ty = (*pt.ty).clone();

	let mut required = false;
	let mut is_all = false;
	let mut default_expr = None;
	let mut set_with_args: Vec<TokenStream> = Vec::new();
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
				synbail!(attr, "`#[prop = ..]` form is not supported")
			}
		};
		let map = AttributeMap::parse(tokens)?;
		for key in map.keys() {
			match key {
				"required" => required = true,
				"all" => is_all = true,
				"default" => default_expr = map.get("default").cloned(),
				// everything else forwards to `#[set_with(..)]`
				other => match map.get(other) {
					Some(expr) => {
						let key = format_ident!("{}", other);
						set_with_args.push(quote! { #key = #expr });
					}
					None => {
						let key = format_ident!("{}", other);
						set_with_args.push(quote! { #key });
					}
				},
			}
		}
	}

	// required props store `Option<ty>`, so the setter must unwrap to `ty`;
	// bare `Option<T>` props get the same ergonomic (setter takes `T`).
	let already_unwraps =
		set_with_args.iter().any(|arg| arg.to_string().contains("unwrap_option"));
	if !already_unwraps && (required || (!required && is_option(&ty))) {
		set_with_args.push(quote! { unwrap_option });
	}

	let prop = Prop {
		ident,
		ty,
		required,
		default_expr,
		other_attrs,
		set_with_args,
	};
	Ok((prop, is_all))
}

/// Whether a parameter is a prop (carries a `#[prop]` attribute).
fn is_prop_param(pt: &syn::PatType) -> bool {
	pt.attrs.iter().any(|attr| attr.path().is_ident("prop"))
}

/// Whether a parameter is the render-context channel: a shared reference to a
/// type named `RouteContext` (`cx: &RouteContext`). The macro wires an ancestor
/// lookup for it rather than treating it as a `SystemParam`.
fn is_route_context_param(pt: &syn::PatType) -> bool {
	let syn::Type::Reference(reference) = pt.ty.as_ref() else {
		return false;
	};
	matches!(
		reference.elem.as_ref(),
		syn::Type::Path(tp)
			if tp.path.segments.last().is_some_and(|seg| seg.ident == "RouteContext")
	)
}

/// Whether a type is `Option<..>`.
fn is_option(ty: &syn::Type) -> bool {
	match ty {
		syn::Type::Path(tp) => tp
			.path
			.segments
			.last()
			.is_some_and(|seg| seg.ident == "Option"),
		_ => false,
	}
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

/// Coerce a typed function argument, rejecting `self`.
fn typed_arg(arg: &FnArg) -> syn::Result<&syn::PatType> {
	match arg {
		FnArg::Typed(pt) => Ok(pt),
		FnArg::Receiver(recv) => {
			synbail!(recv, "`#[scene]` functions cannot take `self`")
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

	let mut props: Vec<Prop> = Vec::new();
	let mut is_all = false;
	for arg in &item.sig.inputs {
		let (prop, all) = parse_prop(typed_arg(arg)?)?;
		is_all |= all;
		props.push(prop);
	}

	let beet_core = pkg_ext::internal_or_beet("beet_core");
	let beet_ui = pkg_ext::internal_or_beet("beet_ui");
	let scene_component_impl = scene_component_impl(fn_name, &props_name);

	// `#[prop(all)]`: the single param's type is the user-defined props type, so
	// no struct/`Default`/`SetWith` is generated — just bind it for the body.
	if is_all {
		if props.len() != 1 {
			synbail!(
				&item.sig.inputs,
				"`#[prop(all)]` must be the only parameter"
			);
		}
		let ident = &props[0].ident;
		return Ok(quote! {
			#(#fn_attrs)*
			#[derive(
				::bevy::ecs::component::Component,
				Default,
				Clone,
				::bevy::reflect::Reflect,
			)]
			#[reflect(Component)]
			#vis struct #fn_name;

			impl #fn_name {
				#[allow(non_snake_case, unused_variables)]
				#vis fn scene(props: #props_name) #output {
					let #ident = props;
					#[allow(unused_braces)]
					#body
				}
			}

			#scene_component_impl
		});
	}

	#[cfg(feature = "slot")]
	append_slot_props(body, &mut props, &beet_ui);

	let field_defs = props.iter().map(Prop::field_def);
	let field_idents: Vec<&syn::Ident> = props.iter().map(|p| &p.ident).collect();
	let props_struct = props_struct(
		vis,
		&props_name,
		&props,
		field_defs,
		&beet_core,
		/* clone */ false,
	);

	let scene_fn = if props.iter().any(|p| p.required) {
		let checks = required_checks(&props, &beet_core);
		let unwraps = required_unwraps(&props);
		quote! {
			#[track_caller]
			#[allow(non_snake_case, unused_variables)]
			#vis fn scene(props: #props_name) #output {
				let location = ::core::panic::Location::caller();
				let #props_name { #(#field_idents),* } = props;
				let mut missing = #beet_core::prelude::Vec::new();
				#(#checks)*
				if !missing.is_empty() {
					return #beet_ui::prelude::SceneExt::any_scene(
						#beet_ui::prelude::ErrorScene::new(
							#beet_ui::prelude::MissingProps { props: missing, location },
						),
					);
				}
				#(#unwraps)*
				#beet_ui::prelude::SceneExt::any_scene(#body)
			}
		}
	} else {
		quote! {
			#[allow(non_snake_case, unused_variables)]
			#vis fn scene(props: #props_name) #output {
				let #props_name { #(#field_idents),* } = props;
				#[allow(unused_braces)]
				#body
			}
		}
	};

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

		#props_struct

		impl #fn_name {
			#scene_fn
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
) -> TokenStream {
	quote! {
		impl ::bevy::scene::SceneComponent for #fn_name {
			type Props = #props_name;
			#[track_caller]
			fn scene(props: Self::Props) -> impl ::bevy::scene::Scene {
				(
					Self::scene(props),
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

/// The props struct definition. Derives `Default` directly unless a
/// `#[prop(default = expr)]` forces a manual `Default` impl. `clone` adds a
/// `Clone` derive (required by `#[scene(system)]`, which clones props into the
/// build closure).
fn props_struct(
	vis: &syn::Visibility,
	props_name: &syn::Ident,
	props: &[Prop],
	field_defs: impl Iterator<Item = TokenStream>,
	beet_core: &syn::Path,
	clone: bool,
) -> TokenStream {
	let field_defs: Vec<_> = field_defs.collect();
	let clone_derive = clone.then(|| quote! { Clone, });
	let needs_manual_default = props.iter().any(|p| p.default_expr.is_some());

	if needs_manual_default {
		let field_idents = props.iter().map(|p| &p.ident);
		let defaults = props.iter().map(Prop::default_value);
		quote! {
			#[derive(#clone_derive #beet_core::prelude::SetWith)]
			#[allow(non_camel_case_types)]
			#vis struct #props_name {
				#(#field_defs),*
			}

			impl ::core::default::Default for #props_name {
				fn default() -> Self {
					Self {
						#(#field_idents: #defaults),*
					}
				}
			}
		}
	} else {
		quote! {
			#[derive(Default, #clone_derive #beet_core::prelude::SetWith)]
			#[allow(non_camel_case_types)]
			#vis struct #props_name {
				#(#field_defs),*
			}
		}
	}
}

/// `if <field>.is_none() { missing.push("<field>"); }` for each required prop.
fn required_checks(props: &[Prop], beet_core: &syn::Path) -> Vec<TokenStream> {
	props
		.iter()
		.filter(|p| p.required)
		.map(|p| {
			let ident = &p.ident;
			let name = syn::LitStr::new(&ident.to_string(), ident.span());
			quote! {
				if #ident.is_none() {
					missing.push(#beet_core::prelude::SmolStr::new_static(#name));
				}
			}
		})
		.collect()
}

/// `let <field> = <field>.unwrap();` for each required prop — after validation
/// each binding matches its originally declared type.
fn required_unwraps(props: &[Prop]) -> Vec<TokenStream> {
	props
		.iter()
		.filter(|p| p.required)
		.map(|p| {
			let ident = &p.ident;
			quote! { let #ident = #ident.unwrap(); }
		})
		.collect()
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

	let mut props: Vec<Prop> = Vec::new();
	let mut sys_types: Vec<TokenStream> = Vec::new();
	let mut sys_pats: Vec<syn::Ident> = Vec::new();
	// the author's `cx: &RouteContext` binding, if any
	let mut cx_binding: Option<syn::Ident> = None;
	for arg in &item.sig.inputs {
		let pt = typed_arg(arg)?;
		if is_prop_param(pt) {
			let (prop, is_all) = parse_prop(pt)?;
			if is_all {
				synbail!(pt, "`#[prop(all)]` is not supported with `#[scene(system)]`");
			}
			props.push(prop);
		} else if is_route_context_param(pt) {
			if cx_binding.is_some() {
				synbail!(pt, "only one `&RouteContext` parameter is supported");
			}
			cx_binding = Some(param_ident(pt)?);
			// fetched via an injected `RenderQuery` ancestor lookup
			sys_types.push(quote! { RenderQuery });
			sys_pats.push(format_ident!("__render_query"));
		} else {
			let ty = &pt.ty;
			sys_types.push(quote! { #ty });
			sys_pats.push(param_ident(pt)?);
		}
	}

	let beet_core = pkg_ext::internal_or_beet("beet_core");
	let beet_ui = pkg_ext::internal_or_beet("beet_ui");
	let scene_component_impl = scene_component_impl(fn_name, &props_name);

	#[cfg(feature = "slot")]
	append_slot_props(body, &mut props, &beet_ui);

	let field_defs = props.iter().map(Prop::field_def);
	let field_idents: Vec<&syn::Ident> = props.iter().map(|p| &p.ident).collect();
	let props_struct = props_struct(
		vis,
		&props_name,
		&props,
		field_defs,
		&beet_core,
		/* clone */ true,
	);

	let unwraps = required_unwraps(&props);
	// when a `cx: &RouteContext` param is present the entity being built is
	// looked up via the injected `RenderQuery`; a missing context surfaces an
	// `ErrorScene` through the build channel (same path as a missing prop). The
	// body must then be type-erased so both arms unify as `Box<dyn Scene>`.
	let closure_body = match &cx_binding {
		Some(cx) => quote! {
			let #props_name { #(#field_idents),* } = props.clone();
			#(#unwraps)*
			let #cx = match __render_query.get_context(_entity) {
				::core::result::Result::Ok(cx) => cx,
				::core::result::Result::Err(err) => {
					return #beet_ui::prelude::SceneExt::any_scene(
						#beet_ui::prelude::ErrorScene::new(err),
					);
				}
			};
			#beet_ui::prelude::SceneExt::any_scene({ #body })
		},
		None => quote! {
			let #props_name { #(#field_idents),* } = props.clone();
			#(#unwraps)*
			#[allow(unused_braces)]
			#body
		},
	};
	let build_closure = quote! {
		#beet_ui::prelude::scene_system::<(#(#sys_types,)*), _, _>(
			move |_entity, (#(#sys_pats,)*)| {
				#closure_body
			},
		)
	};

	// only emit the build-time required check when a prop is required, keeping
	// the zero-overhead path verbatim otherwise.
	let scene_fn = if props.iter().any(|p| p.required) {
		let checks: Vec<TokenStream> = props
			.iter()
			.filter(|p| p.required)
			.map(|p| {
				let ident = &p.ident;
				let name = syn::LitStr::new(&ident.to_string(), ident.span());
				quote! {
					if props.#ident.is_none() {
						missing.push(#beet_core::prelude::SmolStr::new_static(#name));
					}
				}
			})
			.collect();
		quote! {
			#[track_caller]
			#[allow(non_snake_case, unused_variables)]
			#vis fn scene(props: #props_name) #output {
				let location = ::core::panic::Location::caller();
				let mut missing = #beet_core::prelude::Vec::new();
				#(#checks)*
				if !missing.is_empty() {
					return #beet_ui::prelude::SceneExt::any_scene(
						#beet_ui::prelude::ErrorScene::new(
							#beet_ui::prelude::MissingProps { props: missing, location },
						),
					);
				}
				#beet_ui::prelude::SceneExt::any_scene(#build_closure)
			}
		}
	} else {
		quote! {
			#[allow(non_snake_case, unused_variables)]
			#vis fn scene(props: #props_name) #output {
				#build_closure
			}
		}
	};

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

		#props_struct

		impl #fn_name {
			#scene_fn
		}

		#scene_component_impl
	})
}

/// Scan a `#[scene]` body for `<slot>` placeholders and append a `SceneProp`
/// prop for each slot name not already declared as a parameter. `<slot/>` maps
/// to `children`, `<slot name="x"/>` to `x` (`-` lowered to `_`).
#[cfg(feature = "slot")]
fn append_slot_props(
	body: &syn::Block,
	props: &mut Vec<Prop>,
	beet_ui: &syn::Path,
) {
	use quote::ToTokens;
	let mut names = Vec::new();
	collect_slot_names(body.to_token_stream(), &mut names);
	let scene_prop_ty: syn::Type =
		syn::parse_quote! { #beet_ui::prelude::SceneProp };
	for name in names {
		let ident = format_ident!("{}", name.replace('-', "_"));
		if props.iter().any(|prop| prop.ident == ident) {
			continue;
		}
		props.push(Prop {
			ident,
			ty: scene_prop_ty.clone(),
			required: false,
			default_expr: None,
			other_attrs: Vec::new(),
			set_with_args: Vec::new(),
		});
	}
}

/// Walk a token stream collecting `<slot>` names (default `children`), recursing
/// into groups so nested `rsx! { .. }` bodies are covered. A slot tag is a `<`
/// punct immediately followed by a `slot` ident; its `name = "x"` literal, if
/// any, names the prop. Names are deduplicated, first-seen order preserved.
#[cfg(feature = "slot")]
fn collect_slot_names(
	tokens: TokenStream,
	out: &mut Vec<alloc::string::String>,
) {
	use alloc::string::ToString;
	use proc_macro2::TokenTree;

	let is_punct = |tree: Option<&TokenTree>, ch: char| {
		matches!(tree, Some(TokenTree::Punct(punct)) if punct.as_char() == ch)
	};

	let trees: Vec<TokenTree> = tokens.into_iter().collect();
	let mut idx = 0;
	while idx < trees.len() {
		if let TokenTree::Group(group) = &trees[idx] {
			collect_slot_names(group.stream(), out);
			idx += 1;
			continue;
		}
		let is_slot_tag = is_punct(trees.get(idx), '<')
			&& matches!(trees.get(idx + 1), Some(TokenTree::Ident(id)) if id == "slot");
		if !is_slot_tag {
			idx += 1;
			continue;
		}
		// scan attributes up to the closing `>` for a `name = "x"` literal
		let mut name = "children".to_string();
		let mut scan = idx + 2;
		while scan < trees.len() && !is_punct(trees.get(scan), '>') {
			let is_name_attr =
				matches!(&trees[scan], TokenTree::Ident(id) if id == "name")
					&& is_punct(trees.get(scan + 1), '=');
			if is_name_attr {
				if let Some(TokenTree::Literal(lit)) = trees.get(scan + 2) {
					if let syn::Lit::Str(str) = syn::Lit::new(lit.clone()) {
						name = str.value();
					}
				}
			}
			scan += 1;
		}
		if !out.contains(&name) {
			out.push(name);
		}
		idx = scan + 1;
	}
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
		// no required props -> derives Default directly, no validation
		assert!(result.contains("derive (Default"));
		assert!(!result.contains("ErrorScene"));
	}

	#[test]
	fn prop_into_becomes_set_with_into() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Button(#[prop(into)] label: String) -> impl Scene { todo!() }
		});
		assert!(result.contains("# [set_with (into)]"));
	}

	#[test]
	fn required_prop_generates_checked_path() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Field(#[prop(required)] variant: Variant) -> impl Scene { todo!() }
		});
		// stored as Option, setter unwraps it
		assert!(result.contains("Option < Variant >"));
		assert!(result.contains("unwrap_option"));
		// validation + error-scene path
		assert!(result.contains("if variant . is_none ()"));
		assert!(result.contains("MissingProps"));
		assert!(result.contains("ErrorScene"));
		assert!(result.contains("let variant = variant . unwrap ()"));
		// required props alone keep the derived Default (None)
		assert!(result.contains("derive (Default"));
	}

	#[test]
	fn default_expr_generates_manual_default() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Field(#[prop(default = "hi")] placeholder: String) -> impl Scene { todo!() }
		});
		// manual Default impl carrying the expression, not a derive
		assert!(result.contains("impl :: core :: default :: Default for FieldProps"));
		assert!(result.contains("\"hi\""));
		assert!(!result.contains("derive (Default"));
	}

	#[test]
	fn bare_option_prop_setter_unwraps() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Field(name: Option<String>) -> impl Scene { todo!() }
		});
		// optional -> stays Option, but the setter unwraps for ergonomics
		assert!(result.contains("unwrap_option"));
		// not required -> no validation
		assert!(!result.contains("ErrorScene"));
	}

	#[test]
	fn prop_all_skips_props_struct() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Field(#[prop(all)] cfg: FieldProps) -> impl Scene { todo!() }
		});
		// no generated struct; binds the param for the body
		assert!(!result.contains("struct FieldProps"));
		assert!(result.contains("fn scene (props : FieldProps)"));
		assert!(result.contains("let cfg = props"));
		assert!(result.contains("type Props = FieldProps"));
	}

	#[test]
	fn prop_all_rejects_extra_params() {
		let err = parse_err(quote!(), syn::parse_quote! {
			fn Field(#[prop(all)] cfg: FieldProps, other: u32) -> impl Scene { todo!() }
		});
		assert!(err.contains("only parameter"));
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
		assert!(result.contains("move | _entity , (config ,) |"));
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
	fn system_required_prop_checks_before_closure() {
		let result = parse_str(quote!(system), syn::parse_quote! {
			fn Panel(#[prop(required)] role: ColorRole, theme: Res<Theme>) -> impl Scene { todo!() }
		});
		assert!(result.contains("if props . role . is_none ()"));
		assert!(result.contains("ErrorScene"));
		assert!(result.contains("let role = role . unwrap ()"));
	}

	#[test]
	fn system_render_context_param() {
		// `cx: &RouteContext` injects a `RenderQuery` system param, threads the
		// built entity, fetches the context, and errors via `ErrorScene` if absent.
		let result = parse_str(quote!(system), syn::parse_quote! {
			fn Nav(cx: &RouteContext, trees: Query<&RouteTree>) -> impl Scene { todo!() }
		});
		assert!(result.contains("RenderQuery"));
		assert!(result.contains("__render_query . get_context (_entity)"));
		assert!(result.contains("ErrorScene"));
		// the regular system param is preserved alongside the injected query
		assert!(result.contains("Query < & RouteTree >"));
	}

	#[test]
	fn component_not_yet_supported() {
		let err = parse_err(quote!(component), syn::parse_quote! {
			fn Panel(role: u32) -> impl Scene { todo!() }
		});
		assert!(err.contains("not yet implemented"));
	}

	#[cfg(feature = "slot")]
	#[test]
	fn slot_injects_scene_props() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Panel() -> impl Scene {
				rsx! {
					<section>
						<header><slot name="header"/></header>
						<div><slot/></div>
					</section>
				}
			}
		});
		// the named slot and the default slot each become a `SceneProp` field
		assert!(result.contains("header : beet_ui :: prelude :: SceneProp"));
		assert!(result.contains("children : beet_ui :: prelude :: SceneProp"));
	}

	#[cfg(feature = "slot")]
	#[test]
	fn slot_name_dash_lowers_to_underscore() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Panel() -> impl Scene {
				rsx! { <nav><slot name="header-nav"/></nav> }
			}
		});
		assert!(result.contains("header_nav : beet_ui :: prelude :: SceneProp"));
	}

	#[cfg(feature = "slot")]
	#[test]
	fn slot_skips_explicitly_declared_prop() {
		// an author-declared `children` is not duplicated by the `<slot/>` scan
		let result = parse_str(quote!(), syn::parse_quote! {
			fn Panel(children: SceneProp) -> impl Scene {
				rsx! { <div><slot/></div> }
			}
		});
		assert_eq!(result.matches("children :").count(), 1);
	}

	#[test]
	fn rejects_self() {
		let err = parse_err(quote!(), syn::parse_quote! {
			fn Bad(self) -> impl Scene { todo!() }
		});
		assert!(err.contains("self"));
	}
}

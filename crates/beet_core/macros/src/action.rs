//! Implementation of the `#[action]` attribute macro.
//!
//! Turns a plain function into an action component: a struct named after the
//! function, requiring the [`Action`] its handler implements.
//!
//! # Field grammar
//!
//! A `#[field]` parameter becomes a struct field, sharing
//! [`beet_core_shared::prelude::Prop`] with `#[template]`'s `#[prop]`. Fields
//! are read live off the caller entity at call time, so a value edited between
//! calls is observed; a missing component is a loud error naming the type.
//!
//! - **async**: the wrapper clones `Self` off the caller before the body runs.
//! - **system**: the wrapper gains a `Query<&Self>` (`&mut Self` when any field
//!   is `#[field(mut)]`) and forwards the author's system params.
//! - **pure**: lowers the wrapper to a system, since a pure body cannot reach
//!   the world to read its own fields.
//!
//! A `#[field(no_clone)]` lands on the struct but is never bound, for a value
//! too expensive to clone per call; the body reads it through the caller.
//! Fields default to `pub` so a cross-module `rsx!` struct-literal patch
//! resolves, and can declare a narrower visibility.
//!
//! `into_action` detaches from the entity, so it freezes the field values at
//! conversion — a system action included, whose frozen values ride in as
//! system *input* (bevy refuses to cache a non-ZST system) rather than in a
//! closure. Middleware is the exception: its component genuinely lives on the
//! host entity the call names as caller, so it keeps the live-fetching
//! wrapper. A `#[field(mut)]` action emits no `IntoAction` at all, since a
//! detached action has nothing to write back to.
//!
//! # Params
//!
//! An extractor declares a request param by taking it. A `#[param]` parameter
//! (the [`Prop`] grammar: a doc comment and an optional `short = 'x'`) becomes
//! a field of a hidden `{Action}Params` struct, a `Params<T>` parameter takes
//! a whole params group, and an `AnyParams` every param. All are parsed off
//! the request before the body runs, through `?`, so an action taking params
//! returns `Result` and a bad param is the caller's `400`; one naming no input
//! of its own reads them off the `RequestParts`. The same signature declares
//! them: the component's `on_add` pushes their `ParamsDeclaration` into
//! `RouteParams`, or `SubtreeParams` when the input carries a `Next`.
//!
//! # The `on_add` hook
//!
//! Bevy keeps one `on_add` per component, so the macro emits one generated
//! associated fn as the hook, path shaped so the action's `Self` and type
//! params stay in scope. It runs, in order, the author's own `on_add`, the
//! params declaration, the exchange adapter's declaration (a route converting
//! its input and output declares what those conversions read) and the
//! provider-integrity guard.
//!
//! # Metadata
//!
//! A macro cannot test a trait bound, so rather than an attribute declaring
//! how much reflection the types support, `MaybeTyped` probes each of `Self`,
//! `In` and `Out` at the call site and `ActionMeta` takes what came back.
extern crate alloc;
use alloc::string::ToString;
use alloc::vec::Vec;
use beet_core_shared::prelude::*;
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::ItemFn;
use syn::ReturnType;
use syn::Type;
use syn::parse_macro_input;

pub(crate) fn impl_action(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	parse(attr.into(), parse_macro_input!(item as ItemFn))
		.unwrap_or_else(|err| err.into_compile_error())
		.into()
}

fn parse(attr: TokenStream, item: ItemFn) -> syn::Result<TokenStream> {
	// ── 1. Parse attributes ──
	let attrs = AttributeMap::parse(attr)?;
	attrs.assert_types(&[], &[
		"result_out",
		"route",
		"pure",
		"local",
		"no_default",
		"no_clone",
	])?;
	let result_out = attrs.contains_key("result_out");
	let is_pure = attrs.contains_key("pure");
	let is_local = attrs.contains_key("local");
	let has_route = attrs.contains_key("route");
	let no_default = attrs.contains_key("no_default");
	let no_clone = attrs.contains_key("no_clone");
	let route_expr: Option<&syn::Expr> = attrs.get("route");

	// ── 2. Extract function data ──
	let beet_action = pkg_ext::internal_or_beet("beet_action");
	let beet_core = pkg_ext::internal_or_beet("beet_core");
	let bevy = pkg_ext::bevy();
	let vis = &item.vis;
	let fn_name = &item.sig.ident;
	let body = &item.block;
	let generics = &item.sig.generics;
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
	let struct_ty = quote! { #fn_name #ty_generics };
	let fn_attrs = &item.attrs;
	let is_async = item.sig.asyncness.is_some();
	let action_fn_name = action_fn_name(fn_name);
	let wrapper_fn_name = format_ident!("{}_fields", action_fn_name);
	let captured_fn_name = format_ident!("{}_captured", action_fn_name);
	let turbofish = make_turbofish(generics);

	if is_local && !is_async {
		synbail!(&item.sig, "`local` is only valid on async actions");
	}

	// ── 3. Partition parameters into `#[field]`s, params and the rest ──
	let mut fields: Vec<Prop> = Vec::new();
	let mut params = ActionParams::default();
	let mut rest: Vec<&syn::PatType> = Vec::new();
	for arg in &item.sig.inputs {
		let pt = Prop::typed_arg(arg, "action")?;
		if Prop::is_param(pt, "field") {
			fields.push(Prop::parse(pt, "field")?);
		} else if Prop::is_param(pt, "param") {
			params.inline.push(ActionParams::parse_inline(pt)?);
		} else if let Some(group) = ParamsGroup::parse(pt) {
			params.groups.push(group);
		} else if is_wrapper_without_args(&pt.ty, "AnyParams") {
			if params.any.is_some() {
				synbail!(pt, "an action takes `AnyParams` once");
			}
			params.any = Some((*pt.pat).clone());
		} else {
			rest.push(pt);
		}
	}
	let has_fields = fields.iter().any(|field| !field.no_clone);
	let has_component = has_derive(fn_attrs, "Component");
	let has_mut = fields.iter().any(|field| field.mutable);

	if !params.is_empty() && !has_component {
		synbail!(
			&item.sig,
			"`#[param]`, `Params<T>` and `AnyParams` require `#[derive(Component)]`: the params are declared by a component hook"
		);
	}
	if has_fields && !has_component {
		synbail!(
			&item.sig,
			"`#[field]` requires `#[derive(Component)]`: fields are read off the caller entity"
		);
	}
	if has_mut && is_async {
		synbail!(
			&item.sig,
			"`#[field(mut)]` is not valid on an async action: mutable component \
			 access cannot cross an await. Use `cx.caller.get_mut` at a sync point"
		);
	}
	if let Some(field) = fields
		.iter()
		.find(|field| field.mutable && (field.required || field.no_clone))
	{
		synbail!(
			&field.ident,
			"`#[field(mut)]` cannot be combined with `required` or `no_clone`"
		);
	}
	if has_derive(fn_attrs, "Default")
		&& let Some(field) =
			fields.iter().find(|field| field.default_expr.is_some())
	{
		synbail!(
			&field.ident,
			"`#[field(default = ..)]` conflicts with `#[derive(Default)]`"
		);
	}

	// ── 4. Determine the action kind ──
	// async → async action (local → single-threaded), pure → func action,
	// otherwise system action. A pure action carrying fields lowers its
	// entity-facing plumbing to a system, the only sync way to reach the world.
	let is_system_handler = !is_async && !is_pure;
	let provider_is_system = is_system_handler || (is_pure && has_fields);
	let simple_factory = if is_async {
		if is_local {
			quote! { #beet_action::prelude::Action::new_async_local }
		} else {
			quote! { #beet_action::prelude::Action::new_async }
		}
	} else if is_pure {
		quote! { #beet_action::prelude::Action::new_pure }
	} else {
		quote! { #beet_action::prelude::Action::new_system }
	};
	let provider_factory = if provider_is_system {
		quote! { #beet_action::prelude::Action::new_system }
	} else {
		simple_factory.clone()
	};
	let async_kw = if is_async {
		quote! { async }
	} else {
		TokenStream::default()
	};

	// ── 5. Analyze the remaining parameters ──
	// an action taking params with no input of its own reads them off the
	// request parts, which need no body
	let beet_net = pkg_ext::internal_or_beet("beet_net");
	let inferred_input = (!params.is_empty())
		.then(|| quote! { #beet_net::prelude::RequestParts });
	let parts = if is_system_handler {
		FnParts::system(&rest, inferred_input, &beet_action)?
	} else {
		FnParts::simple(&rest, inferred_input, &item, &beet_action)?
	};
	let FnParts {
		in_type,
		fn_params,
		preamble,
		sys_params,
		cx,
	} = &parts;
	let out_type = compute_out_type(&item, result_out);
	let return_type = match &item.sig.output {
		ReturnType::Default => quote! { () },
		ReturnType::Type(_, ty) => quote! { #ty },
	};
	// the handler's own result shape: a `Result` return already carries the
	// wrapper's error channel, anything else (including a `result_out` action,
	// whose `Out` *is* the `Result`) is lifted into one.
	let returns_result = match &item.sig.output {
		ReturnType::Type(_, ty) => is_result_type(ty),
		ReturnType::Default => false,
	};
	let lift_ok = !(returns_result && !result_out);
	if !params.is_empty() && !returns_result {
		synbail!(
			&item.sig,
			"an action taking params returns `Result`, so a bad param is the caller's `400`"
		);
	}
	let params_ident = format_ident!("{}Params", fn_name);
	let params_struct = params.make_struct(&params_ident, &beet_net, &bevy);
	let params_reads = params.make_reads(&params_ident, cx, &beet_net);

	// ── 6. Build the struct definition ──
	// the meta rides on the action itself: `Action` is the sole producer of
	// `ActionMeta`, and naming `Self` as the handler is what the provider-integrity
	// guard checks against.
	//
	// Middleware (a `Next` in the input) wraps *another* entity's action rather
	// than being one, and several middleware happily share a host, so it claims no
	// action slot: its `MiddlewareList::on_add` hook pushes `into_action()` onto the
	// host's `MiddlewareList` instead.
	let is_middleware = has_next_type(in_type);
	let meta_expr =
		make_meta_expr(&struct_ty, in_type, &out_type, &beet_action);
	let provider_target = match has_fields {
		true => quote! { #wrapper_fn_name #turbofish },
		false => quote! { #action_fn_name #turbofish },
	};
	let action_expr = quote! {
		#provider_factory(#provider_target).with_meta(#meta_expr)
	};
	let require_action = (!is_middleware).then(|| {
		make_require_action(
			action_expr,
			in_type,
			&out_type,
			has_route,
			&beet_action,
		)
	});
	let mut fn_attrs = fn_attrs.to_vec();
	let on_add = OnAdd {
		declare_params: params.make_declare(
			&params_ident,
			is_middleware,
			&beet_net,
		),
		declare_exchange: (has_component && has_route && !is_response(&out_type))
			.then(|| {
				let beet_router = pkg_ext::internal_or_beet("beet_router");
				quote! {
					#beet_router::prelude::route::declare_exchange::<#in_type, #out_type, _, _>(world.reborrow(), cx);
				}
			}),
		guard: (has_component && require_action.is_some()).then(|| {
			quote! {
				#beet_action::prelude::Action::<#in_type, #out_type>::assert_provider::<Self>(world.reborrow(), cx);
			}
		}),
		author: None,
	}
	.take_author(&mut fn_attrs);
	let struct_def = make_struct_def(
		vis,
		fn_name,
		generics,
		&fn_attrs,
		&fields,
		require_action,
		on_add.attr(),
		route_expr,
	)?;
	let on_add_fn = on_add.make_fn(fn_name, generics, &bevy);
	let clone_impl = (!no_clone && !has_derive(&fn_attrs, "Clone"))
		.then(|| make_clone(fn_name, generics, &fields));
	let default_impl = (!no_default && !has_derive(&fn_attrs, "Default"))
		.then(|| make_default(fn_name, generics, &fields));

	// ── 7. Build the handler function at module level ──
	// The handler must live at module scope so that both `IntoAction`
	// and `#[require]` (generated by the Component derive) can
	// reference it.
	let field_params =
		fields.iter().filter(|field| !field.no_clone).map(|field| {
			let ident = &field.ident;
			let ty = &field.ty;
			match field.mutable {
				true => quote! { #ident: &mut #ty },
				false => quote! { #ident: #ty },
			}
		});
	let handler_fn = quote! {
		#[allow(non_snake_case)]
		#async_kw fn #action_fn_name #impl_generics (
			#(#field_params,)* #fn_params
		) -> #return_type #where_clause {
			#params_reads
			#preamble
			#[allow(unused_braces)]
			#body
		}
	};

	// ── 8. Build the live-fetching wrapper and the `IntoAction` impl ──
	let result_ty = quote! {
		::core::result::Result<#out_type, #bevy::ecs::error::BevyError>
	};
	// a `no_clone` field is on the struct but never bound: the body reaches it
	// through the caller itself rather than paying a clone per call.
	let bound: Vec<&Prop> =
		fields.iter().filter(|field| !field.no_clone).collect();
	let field_idents: Vec<&syn::Ident> =
		bound.iter().map(|field| &field.ident).collect();
	let field_checks = field_checks(&bound, &struct_ty, &beet_core);
	let call_handler = |args: TokenStream| {
		let await_kw = is_async.then(|| quote! { .await });
		let call = quote! { #action_fn_name #turbofish (#args) #await_kw };
		match lift_ok {
			true => quote! { ::core::result::Result::Ok(#call) },
			false => call,
		}
	};

	// gated with the `IntoAction` impl below: a `mut` action has nothing to
	// write back to once detached, so it never freezes its fields.
	let captured_fn = (has_fields
		&& !is_middleware
		&& !has_mut
		&& is_system_handler)
		.then(|| {
			let (impl_generics, _, where_clause) = generics.split_for_impl();
			let sys_declared = sys_params
				.iter()
				.map(SysParam::declare_forwarding)
				.collect::<Vec<_>>();
			let sys_forwarded = sys_params.iter().map(SysParam::forward);
			let call = call_handler(quote! {
				#(#field_idents,)* #bevy::prelude::In(__action_cx)
				#(, #sys_forwarded)*
			});
			quote! {
				#[allow(non_snake_case, dead_code)]
				fn #captured_fn_name #impl_generics (
					#bevy::prelude::In((__action_fields, __action_cx)):
						#bevy::prelude::In<(
							#struct_ty,
							#beet_action::prelude::ActionContext<#in_type>,
						)>
					#(, #sys_declared)*
				) -> #result_ty #where_clause {
					let #fn_name { #(#field_idents,)* .. } = __action_fields;
					#(#field_checks)*
					#call
				}
			}
		});

	let wrapper_fn = has_fields.then(|| {
		make_wrapper(WrapperArgs {
			wrapper_fn_name: &wrapper_fn_name,
			generics,
			struct_ty: &struct_ty,
			fields: &bound,
			field_idents: &field_idents,
			field_checks: &field_checks,
			in_type,
			result_ty: &result_ty,
			sys_params,
			provider_is_system,
			is_system_handler,
			call_handler: &call_handler,
			beet_action: &beet_action,
			bevy: &bevy,
		})
	});

	// a detached action has no component to read, so it freezes the fields at
	// conversion. Middleware is the exception: its component lives on the host
	// entity the call names as caller, so it keeps reading live.
	let capture_fields = has_fields && !is_middleware;
	let into_action_body = if !has_fields {
		quote! { #simple_factory(#action_fn_name #turbofish) }
	} else if !capture_fields {
		quote! { #simple_factory(#wrapper_fn_name #turbofish) }
	} else if is_system_handler {
		// bevy refuses to cache a non-ZST system, so the frozen fields ride in
		// as input rather than in a closure.
		quote! {
			#beet_action::prelude::Action::new_system_with(self, #captured_fn_name #turbofish)
		}
	} else {
		let call = call_handler(quote! { #(#field_idents,)* __action_cx });
		let inner = quote! {
			let #fn_name { #(#field_idents,)* .. } = __action_fields.clone();
			#(#field_checks)*
			#call
		};
		let body = match is_async {
			true => quote! {
				let __action_fields = __action_fields.clone();
				async move { #inner }
			},
			false => inner,
		};
		quote! {
			let __action_fields = self;
			#simple_factory(
				move |__action_cx: #beet_action::prelude::ActionContext<#in_type>| {
					#body
				}
			)
		}
	};
	let into_action = (!has_mut).then(|| {
		quote! {
			impl #impl_generics #beet_action::prelude::IntoAction<#struct_ty> for #struct_ty #where_clause {
				type In = #in_type;
				type Out = #out_type;

				fn into_action(self) -> #beet_action::prelude::Action<Self::In, Self::Out> {
					#into_action_body
				}
			}
		}
	});

	// ── 9. Assemble output ──
	Ok(quote! {
		#params_struct
		#handler_fn
		#wrapper_fn
		#captured_fn
		#struct_def
		#on_add_fn
		#clone_impl
		#default_impl
		#into_action
	})
}

// ---------------------------------------------------------------------------
// Parameter analysis
// ---------------------------------------------------------------------------

/// The handler's non-`#[field]` parameters, lowered.
struct FnParts {
	/// the action's `In` type
	in_type: TokenStream,
	/// the handler's parameter list
	fn_params: TokenStream,
	/// binds the author's input pattern before the body
	preamble: TokenStream,
	/// system params a field wrapper must declare and forward, keeping the
	/// author's attributes (a `#[cfg]` param exists in only some builds) and
	/// binding pattern (so a `mut` binding stays mutable)
	sys_params: Vec<SysParam>,
	/// the handler's binding of its `ActionContext`, which params are read
	/// through
	cx: syn::Ident,
}

/// One non-`#[field]` system parameter, as the handler declares it.
struct SysParam {
	/// the author's attributes, ie a `#[cfg]` gating the param
	attrs: Vec<syn::Attribute>,
	/// the binding pattern, preserving `mut`
	pat: syn::PatIdent,
	ty: Type,
}

impl SysParam {
	/// The handler's declaration, verbatim.
	fn declare(&self) -> TokenStream {
		let Self { attrs, pat, ty } = self;
		quote! { #(#attrs)* #pat: #ty }
	}

	/// The wrapper's declaration: the same param bound immutably, since a
	/// wrapper only forwards it.
	fn declare_forwarding(&self) -> TokenStream {
		let Self { attrs, pat, ty } = self;
		let ident = &pat.ident;
		quote! { #(#attrs)* #ident: #ty }
	}

	/// The wrapper's call argument, gated by the same attributes as the
	/// declaration it forwards.
	fn forward(&self) -> TokenStream {
		let Self { attrs, pat, .. } = self;
		let ident = &pat.ident;
		quote! { #(#attrs)* #ident }
	}
}

impl FnParts {
	/// Build the parts for async and pure actions, which accept at most one
	/// non-field parameter: the input.
	fn simple(
		params: &[&syn::PatType],
		inferred_input: Option<TokenStream>,
		item: &ItemFn,
		beet_action: &syn::Path,
	) -> syn::Result<Self> {
		if params.len() > 1 {
			synbail!(
				&item.sig,
				"action functions accept at most one parameter; \
				 use a tuple for multiple values: `(a, b): (A, B)`"
			);
		}

		let parts = match (params.first(), inferred_input) {
			// No params, but an input inferred from what the action takes
			(None, Some(input)) => Self {
				in_type: input.clone(),
				fn_params: quote! {
					__action_cx: #beet_action::prelude::ActionContext<#input>
				},
				preamble: TokenStream::default(),
				sys_params: Vec::new(),
				cx: action_cx(),
			},
			(None, None) => {
				// No params → input is ()
				Self {
					in_type: quote! { () },
					fn_params: quote! {
						__action_cx: #beet_action::prelude::ActionContext
					},
					preamble: quote! { let _ = __action_cx.input; },
					sys_params: Vec::new(),
					cx: action_cx(),
				}
			}
			(Some(pt), _) => {
				let ty = pt.ty.as_ref();
				if let Some(inner) = extract_action_context_type(ty) {
					// Passthrough: user gets the full ActionContext
					let param_name = pat_to_ident(&pt.pat)?;
					Self {
						in_type: quote! { #inner },
						fn_params: quote! {
							#param_name: #beet_action::prelude::ActionContext<#inner>
						},
						preamble: TokenStream::default(),
						sys_params: Vec::new(),
						cx: param_name,
					}
				} else {
					// Bare input: destructure from context
					let pat = &pt.pat;
					Self {
						in_type: quote! { #ty },
						fn_params: quote! {
							__action_cx: #beet_action::prelude::ActionContext<#ty>
						},
						preamble: quote! { let #pat = __action_cx.input; },
						sys_params: Vec::new(),
						cx: action_cx(),
					}
				}
			}
		};
		Ok(parts)
	}

	/// Build the parts for system actions.
	///
	/// Detects input via `In<T>` or `ActionContext<T>` on the first non-field
	/// parameter; the remainder are forwarded as system params.
	fn system(
		params: &[&syn::PatType],
		inferred_input: Option<TokenStream>,
		beet_action: &syn::Path,
	) -> syn::Result<Self> {
		// with no input marker of its own, the context of the inferred input,
		// else of `()`
		let no_input = |sys_params: Vec<SysParam>| match &inferred_input {
			Some(input) => (
				input.clone(),
				quote! {
					In(__action_cx): In<#beet_action::prelude::ActionContext<#input>>
				},
				TokenStream::default(),
				sys_params,
				action_cx(),
			),
			None => (
				quote! { () },
				quote! {
					In(__action_cx): In<#beet_action::prelude::ActionContext>
				},
				quote! { let _ = __action_cx.input; },
				sys_params,
				action_cx(),
			),
		};
		let sys_params_from = |skip: usize| -> Vec<SysParam> {
			params
				.iter()
				.skip(skip)
				.enumerate()
				.map(|(index, pt)| {
					let pat = match pt.pat.as_ref() {
						syn::Pat::Ident(pi) => pi.clone(),
						_ => syn::PatIdent {
							attrs: Vec::new(),
							by_ref: None,
							mutability: None,
							ident: format_ident!("__action_sys_{}", index),
							subpat: None,
						},
					};
					SysParam {
						attrs: pt.attrs.clone(),
						pat,
						ty: (*pt.ty).clone(),
					}
				})
				.collect()
		};

		let (in_type, first_fn_param, preamble, sys_params, cx) = match params
			.first()
		{
			None => no_input(Vec::new()),
			Some(pt) => {
				let ty = pt.ty.as_ref();

				if let Some(in_inner) = extract_wrapper_type(ty, "In") {
					if let Some(inner) = extract_action_context_type(in_inner) {
						// In<ActionContext<T>> → system passthrough
						let param_name = pat_to_ident(&pt.pat)?;
						(
							quote! { #inner },
							quote! {
								In(#param_name): In<#beet_action::prelude::ActionContext<#inner>>
							},
							TokenStream::default(),
							sys_params_from(1),
							param_name,
						)
					} else {
						// In<T> → system action with input T
						let param_name = pat_to_ident(&pt.pat)?;
						(
							quote! { #in_inner },
							quote! {
								In(__action_cx): In<#beet_action::prelude::ActionContext<#in_inner>>
							},
							quote! { let #param_name = In(__action_cx.input); },
							sys_params_from(1),
							action_cx(),
						)
					}
				} else if let Some(inner) = extract_action_context_type(ty) {
					// ActionContext<T> without In wrapper → system passthrough
					let param_name = pat_to_ident(&pt.pat)?;
					(
						quote! { #inner },
						quote! {
							In(#param_name): In<#beet_action::prelude::ActionContext<#inner>>
						},
						TokenStream::default(),
						sys_params_from(1),
						param_name,
					)
				} else {
					// No input marker → all params are system params
					no_input(sys_params_from(0))
				}
			}
		};

		// re-declare the system params from the resolved patterns so a wrapper can
		// forward them by name, keeping the two signatures in step.
		let declared =
			sys_params.iter().map(SysParam::declare).collect::<Vec<_>>();
		let fn_params = quote! { #first_fn_param #(, #declared)* };

		Ok(Self {
			in_type,
			fn_params,
			preamble,
			sys_params,
			cx,
		})
	}
}

/// The handler's default binding of its `ActionContext`.
fn action_cx() -> syn::Ident {
	syn::Ident::new("__action_cx", proc_macro2::Span::call_site())
}

// ---------------------------------------------------------------------------
// Params
// ---------------------------------------------------------------------------

/// The request params an action takes, see the [module](self) docs.
#[derive(Default)]
struct ActionParams {
	/// `#[param]` parameters, the fields of the hidden `{Action}Params` struct.
	inline: Vec<InlineParam>,
	/// `Params<T>` parameters, in argument order.
	groups: Vec<ParamsGroup>,
	/// The binding of an `AnyParams` parameter, every param the request
	/// carries.
	any: Option<syn::Pat>,
}

/// One `#[param]` parameter.
struct InlineParam {
	/// The field the hidden struct declares.
	prop: Prop,
	/// The author's binding, preserving `mut`.
	pat: syn::PatIdent,
}

/// One `Params<T>` parameter.
struct ParamsGroup {
	/// The author's binding pattern.
	pat: syn::Pat,
	/// The `T` of `Params<T>`.
	ty: Type,
}

impl ParamsGroup {
	/// The group a `Params<T>` parameter takes, `None` for any other type.
	fn parse(pt: &syn::PatType) -> Option<Self> {
		let ty = extract_wrapper_type(&pt.ty, "Params")?;
		Some(Self {
			pat: (*pt.pat).clone(),
			ty: ty.clone(),
		})
	}
}

impl ActionParams {
	/// Parse a `#[param]` parameter, which takes a doc comment and a `short`.
	fn parse_inline(pt: &syn::PatType) -> syn::Result<InlineParam> {
		let prop = Prop::parse(pt, "param")?;
		if prop.required
			|| prop.mutable
			|| prop.no_clone
			|| prop.default_expr.is_some()
		{
			synbail!(
				&prop.ident,
				"a `#[param]` takes only `short = 'x'`: its type alone says whether it is required"
			);
		}
		Ok(InlineParam {
			pat: Prop::param_pat_ident(pt, "param")?,
			prop,
		})
	}

	/// Whether the action takes no params.
	fn is_empty(&self) -> bool {
		self.inline.is_empty() && self.groups.is_empty() && self.any.is_none()
	}

	/// The hidden `{Action}Params` struct holding the `#[param]` fields, each
	/// keeping its doc comment as its `--help` description.
	fn make_struct(
		&self,
		ident: &syn::Ident,
		beet_net: &syn::Path,
		bevy: &syn::Path,
	) -> Option<TokenStream> {
		if self.inline.is_empty() {
			return None;
		}
		let fields = self.inline.iter().map(|InlineParam { prop, .. }| {
			let Prop {
				ident,
				ty,
				short,
				other_attrs,
				..
			} = prop;
			let short = short.as_ref().map(|short| {
				quote! { #[reflect(@#beet_net::prelude::ParamOptions::short(#short))] }
			});
			quote! {
				#(#other_attrs)*
				#short
				#ident: #ty
			}
		});
		quote! {
			#[derive(#bevy::reflect::Reflect)]
			#[doc(hidden)]
			#[allow(non_camel_case_types)]
			struct #ident {
				#(#fields,)*
			}
		}
		.into()
	}

	/// The reads ahead of the body: the `#[param]` fields destructured out of
	/// the hidden struct, then each `Params<T>` in argument order.
	fn make_reads(
		&self,
		ident: &syn::Ident,
		cx: &syn::Ident,
		beet_net: &syn::Path,
	) -> TokenStream {
		let parts = quote! {
			#beet_net::prelude::RequestPartsRef::request_parts(&#cx.input)
		};
		let inline = (!self.inline.is_empty()).then(|| {
			// shorthand, the author's binding being the field's own name
			let bindings = self
				.inline
				.iter()
				.map(|InlineParam { pat, .. }| quote! { #pat });
			quote! {
				let #ident { #(#bindings,)* } = #parts.parse_params::<#ident>()?;
			}
		});
		let groups = self.groups.iter().map(|ParamsGroup { pat, ty }| {
			quote! {
				let #pat = #beet_net::prelude::Params(#parts.parse_params::<#ty>()?);
			}
		});
		let any = self.any.as_ref().map(|pat| {
			quote! {
				let #pat = #beet_net::prelude::AnyParams::from(#parts);
			}
		});
		quote! { #inline #(#groups)* #any }
	}

	/// The `on_add` step declaring every param the action takes, the hidden
	/// struct, each group, then any `AnyParams`: route scope for an action,
	/// subtree scope when the input carries a `Next`.
	fn make_declare(
		&self,
		ident: &syn::Ident,
		is_middleware: bool,
		beet_net: &syn::Path,
	) -> Option<TokenStream> {
		if self.is_empty() {
			return None;
		}
		let types = (!self.inline.is_empty())
			.then(|| quote! { #ident })
			.into_iter()
			.chain(self.groups.iter().map(|group| {
				let ty = &group.ty;
				quote! { #ty }
			}))
			.chain(
				self.any
					.as_ref()
					.map(|_| quote! { #beet_net::prelude::AnyParams }),
			);
		let list = match is_middleware {
			true => quote! { SubtreeParams },
			false => quote! { RouteParams },
		};
		quote! {
			#beet_net::prelude::#list::declare::<Self, (#(#types,)*)>(world.reborrow(), cx);
		}
		.into()
	}
}

/// Whether an output type is already a `Response`, needing no conversion.
fn is_response(out_type: &TokenStream) -> bool { is_type(out_type, "Response") }

/// Whether `ty` is the plain type `name`, by its last path segment.
fn is_type(ty: &TokenStream, name: &str) -> bool {
	syn::parse2::<Type>(ty.clone())
		.is_ok_and(|ty| is_wrapper_without_args(&ty, name))
}

// ---------------------------------------------------------------------------
// Field wrapper
// ---------------------------------------------------------------------------

/// Everything [`make_wrapper`] needs to emit the live-fetching wrapper.
struct WrapperArgs<'a> {
	wrapper_fn_name: &'a syn::Ident,
	generics: &'a syn::Generics,
	struct_ty: &'a TokenStream,
	fields: &'a [&'a Prop],
	field_idents: &'a [&'a syn::Ident],
	field_checks: &'a [TokenStream],
	in_type: &'a TokenStream,
	result_ty: &'a TokenStream,
	sys_params: &'a [SysParam],
	provider_is_system: bool,
	is_system_handler: bool,
	call_handler: &'a dyn Fn(TokenStream) -> TokenStream,
	beet_action: &'a syn::Path,
	bevy: &'a syn::Path,
}

/// The wrapper the `#[require]` site installs: read `Self` off the caller,
/// bind its fields, then call the handler.
fn make_wrapper(args: WrapperArgs) -> TokenStream {
	let WrapperArgs {
		wrapper_fn_name,
		generics,
		struct_ty,
		fields,
		field_idents,
		field_checks,
		in_type,
		result_ty,
		sys_params,
		provider_is_system,
		is_system_handler,
		call_handler,
		beet_action,
		bevy,
	} = args;
	let (impl_generics, _, where_clause) = generics.split_for_impl();

	// only the bound fields are cloned: a `no_clone` field stays on the entity
	let reads = field_idents.iter().map(|ident| {
		quote! { ::core::clone::Clone::clone(&__action_item.#ident) }
	});

	if !provider_is_system {
		// async: clone the bound fields off the caller before the body runs
		let call = call_handler(quote! { #(#field_idents,)* __action_cx });
		return quote! {
			#[allow(non_snake_case, dead_code)]
			async fn #wrapper_fn_name #impl_generics (
				__action_cx: #beet_action::prelude::ActionContext<#in_type>,
			) -> #result_ty #where_clause {
				let (#(#field_idents,)*) = __action_cx
					.caller
					.get::<#struct_ty, _>(|__action_item| (#(#reads,)*))
					.await?;
				#(#field_checks)*
				#call
			}
		};
	}

	let has_mut = fields.iter().any(|field| field.mutable);
	// a `mut` field needs the mutable query, and reads clone before any field is
	// reborrowed so the two never overlap.
	let (query_param, bindings) = if has_mut {
		let reads = fields.iter().filter(|field| !field.mutable).map(|field| {
			let ident = &field.ident;
			quote! { let #ident = ::core::clone::Clone::clone(&__action_item.#ident); }
		});
		let writes = fields.iter().filter(|field| field.mutable).map(|field| {
			let ident = &field.ident;
			quote! { let #ident = &mut __action_item.#ident; }
		});
		(
			quote! { mut __action_query: #bevy::ecs::system::Query<&mut #struct_ty> },
			quote! {
				let ::core::result::Result::Ok(__action_item) =
					__action_query.get_mut(__action_cx.id())
				else {
					return ::core::result::Result::Err(
						__action_cx.missing_component::<#struct_ty>()
					);
				};
				let __action_item = __action_item.into_inner();
				#(#reads)*
				#(#writes)*
			},
		)
	} else {
		(
			quote! { __action_query: #bevy::ecs::system::Query<&#struct_ty> },
			quote! {
				let ::core::result::Result::Ok(__action_item) =
					__action_query.get(__action_cx.id())
				else {
					return ::core::result::Result::Err(
						__action_cx.missing_component::<#struct_ty>()
					);
				};
				#(let #field_idents = #reads;)*
			},
		)
	};

	// a pure handler takes the bare context, a system handler the `In`-wrapped one
	let cx_arg = match is_system_handler {
		true => quote! { #bevy::prelude::In(__action_cx) },
		false => quote! { __action_cx },
	};
	let sys_forwarded = sys_params.iter().map(SysParam::forward);
	// the wrapper only forwards, so it declares each param immutably
	let sys_declared = sys_params
		.iter()
		.map(SysParam::declare_forwarding)
		.collect::<Vec<_>>();
	let call = call_handler(
		quote! { #(#field_idents,)* #cx_arg #(, #sys_forwarded)* },
	);
	quote! {
		#[allow(non_snake_case, dead_code)]
		fn #wrapper_fn_name #impl_generics (
			#bevy::prelude::In(__action_cx):
				#bevy::prelude::In<#beet_action::prelude::ActionContext<#in_type>>,
			#query_param
			#(, #sys_declared)*
		) -> #result_ty #where_clause {
			#bindings
			#(#field_checks)*
			#call
		}
	}
}

/// The per-field preamble shared by every binding site: a required field is
/// unwrapped, erroring loudly by name.
fn field_checks(
	fields: &[&Prop],
	struct_ty: &TokenStream,
	beet_core: &syn::Path,
) -> Vec<TokenStream> {
	fields
		.iter()
		.filter(|field| field.required)
		.map(|field| {
			let ident = &field.ident;
			let lit = syn::LitStr::new(&ident.to_string(), ident.span());
			quote! {
				let ::core::option::Option::Some(#ident) = #ident
				else {
					return ::core::result::Result::Err(#beet_core::prelude::bevyhow!(
						"{}: missing required field `{}`",
						::core::any::type_name::<#struct_ty>(),
						#lit
					));
				};
			}
		})
		.collect()
}

// ---------------------------------------------------------------------------
// Type extraction helpers
// ---------------------------------------------------------------------------

/// Extract inner type, recognizing `ActionContext<T>` and its default-unit form (no generic args → `()`).
fn extract_action_context_type(ty: &Type) -> Option<Type> {
	extract_wrapper_type_or_unit(ty, "ActionContext")
}

/// Extract an identifier from a pattern.
///
/// Returns the ident for `Pat::Ident`, or a generated discard name for
/// `Pat::Wild`. Errors on complex patterns (tuples, structs, etc.).
fn pat_to_ident(pat: &syn::Pat) -> syn::Result<syn::Ident> {
	match pat {
		syn::Pat::Ident(pi) => Ok(pi.ident.clone()),
		syn::Pat::Wild(_) => Ok(syn::Ident::new(
			"__action_discard",
			proc_macro2::Span::call_site(),
		)),
		_ => synbail!(pat, "expected an identifier or `_`"),
	}
}

/// Extract the inner type `T` from a wrapper type `Wrapper<T>`, matching
/// only on the last path segment name.
fn extract_wrapper_type<'a>(ty: &'a Type, name: &str) -> Option<&'a Type> {
	if let Type::Path(type_path) = ty {
		if let Some(segment) = type_path.path.segments.last() {
			if segment.ident == name {
				if let syn::PathArguments::AngleBracketed(args) =
					&segment.arguments
				{
					if let Some(syn::GenericArgument::Type(inner)) =
						args.args.first()
					{
						return Some(inner);
					}
				}
			}
		}
	}
	None
}

/// Extract inner type for wrappers with default generic unit:
/// `Wrapper<T>` -> `T`, `Wrapper` -> `()`.
fn extract_wrapper_type_or_unit(ty: &Type, name: &str) -> Option<Type> {
	if let Some(inner) = extract_wrapper_type(ty, name) {
		Some(inner.clone())
	} else if is_wrapper_without_args(ty, name) {
		Some(syn::parse_quote! { () })
	} else {
		None
	}
}

/// Whether a type path is `Wrapper` with no generic args.
fn is_wrapper_without_args(ty: &Type, name: &str) -> bool {
	if let Type::Path(type_path) = ty {
		if let Some(segment) = type_path.path.segments.last() {
			return segment.ident == name
				&& matches!(segment.arguments, syn::PathArguments::None);
		}
	}
	false
}

/// Whether the return type path ends with `Result`.
fn is_result_type(ty: &Type) -> bool {
	if let Type::Path(type_path) = ty {
		if let Some(segment) = type_path.path.segments.last() {
			return segment.ident == "Result";
		}
	}
	false
}

/// Whether the input type contains a `Next` type, indicating middleware.
fn has_next_type(tokens: &TokenStream) -> bool {
	for tt in tokens.clone().into_iter() {
		match tt {
			proc_macro2::TokenTree::Ident(ident) if ident == "Next" => {
				return true;
			}
			proc_macro2::TokenTree::Group(group) => {
				if has_next_type(&group.stream()) {
					return true;
				}
			}
			_ => {}
		}
	}
	false
}

/// Extract the inner `T` from `Result<T>` or `Result<T, E>`.
fn extract_result_inner(ty: &Type) -> Option<&Type> {
	extract_wrapper_type(ty, "Result")
}

// ---------------------------------------------------------------------------
// Output type and body helpers
// ---------------------------------------------------------------------------

/// Compute the output type from the function signature.
fn compute_out_type(item: &ItemFn, result_out: bool) -> TokenStream {
	let raw_return_type: Option<&Type> = match &item.sig.output {
		ReturnType::Default => None,
		ReturnType::Type(_, ty) => Some(ty.as_ref()),
	};

	let returns_result = raw_return_type
		.map(|ty| is_result_type(ty))
		.unwrap_or(false);

	if let Some(ty) = raw_return_type {
		if returns_result && !result_out {
			if let Some(inner) = extract_result_inner(ty) {
				quote! { #inner }
			} else {
				quote! { () }
			}
		} else {
			quote! { #ty }
		}
	} else {
		quote! { () }
	}
}

// ---------------------------------------------------------------------------
// Struct definition and require helpers
// ---------------------------------------------------------------------------

/// Build the `ActionMeta` expression threaded into the action.
///
/// A macro cannot test a trait bound, so rather than making the author declare
/// how much reflection their types support, each of `Self`, `In` and `Out` is
/// probed with `MaybeTyped` and the meta takes whatever comes back: a doc
/// description from a reflecting handler, a schema from a reflecting input or
/// output, nothing where the bound does not hold.
fn make_meta_expr(
	struct_ty: &TokenStream,
	in_type: &TokenStream,
	out_type: &TokenStream,
	beet_action: &syn::Path,
) -> TokenStream {
	let prelude = quote! { #beet_action::prelude };
	let probe = |ty: &TokenStream| {
		quote! { (&&#prelude::MaybeTyped::<#ty>::new()).maybe_type_info() }
	};
	let (handler, input, output) =
		(probe(struct_ty), probe(in_type), probe(out_type));
	quote! {
		{
			// both arms of the ladder must be nameable for method resolution to
			// choose between them
			#[allow(unused_imports)]
			use #prelude::MaybeTypedFallback as _;
			#[allow(unused_imports)]
			use #prelude::MaybeTypedReflect as _;
			#prelude::ActionMeta::of::<#struct_ty, #in_type, #out_type>()
				.with_type_info(#handler, #input, #output)
		}
	}
}

/// Build the `#[require(...)]` expression for the `Action` component.
///
/// When `has_route` is true, requires both `Action<In, Out>` and an
/// [`ActionOverload<Request, Response>`] adapting the typed action to
/// request/response dispatch, unless the action already is one.
fn make_require_action(
	action_expr: TokenStream,
	in_type: &TokenStream,
	out_type: &TokenStream,
	has_route: bool,
	beet_action: &syn::Path,
) -> TokenStream {
	// a `Request -> Response` handler is its own route action, so an adapter
	// would never be invoked, and skipping it keeps the attribute usable in a
	// crate beneath beet_router
	if has_route && !(is_type(in_type, "Request") && is_response(out_type)) {
		let beet_router = pkg_ext::internal_or_beet("beet_router");
		quote! {
			#beet_action::prelude::Action<#in_type, #out_type> = #action_expr,
			#beet_router::prelude::ExchangeOverload = #beet_router::prelude::route::exchange_overload::<#in_type, #out_type, _, _>()
		}
	} else {
		quote! {
			#beet_action::prelude::Action<#in_type, #out_type> = #action_expr
		}
	}
}

/// Generate a struct definition, forwarding function attributes to the
/// struct and optionally adding `#[require(...)]` attributes when the
/// derives include `Component`.
#[allow(clippy::too_many_arguments)]
fn make_struct_def(
	vis: &syn::Visibility,
	fn_name: &syn::Ident,
	generics: &syn::Generics,
	fn_attrs: &[syn::Attribute],
	fields: &[Prop],
	require_action: Option<TokenStream>,
	on_add_attr: TokenStream,
	route_expr: Option<&syn::Expr>,
) -> syn::Result<TokenStream> {
	let has_component = has_derive(fn_attrs, "Component");

	let require_action = match (has_component, require_action) {
		(true, Some(expr)) => quote! { #[require(#expr)] },
		_ => TokenStream::default(),
	};

	let require_path = if has_component && let Some(expr) = route_expr {
		let beet_net = pkg_ext::internal_or_beet("beet_net");
		quote! {
			#[require(#beet_net::prelude::PathPartial = #beet_net::prelude::PathPartial::new(#expr))]
		}
	} else {
		TokenStream::default()
	};

	let field_defs: Vec<TokenStream> = fields
		.iter()
		.map(Prop::field_def)
		.chain(marker_field(generics, fn_attrs))
		.collect();

	let (_, _, where_clause) = generics.split_for_impl();
	let body = if field_defs.is_empty() {
		quote! { ; }
	} else {
		quote! { { #(#field_defs),* } }
	};
	Ok(quote! {
		#(#fn_attrs)*
		#require_action
		#on_add_attr
		#require_path
		#[allow(non_camel_case_types)]
		#vis struct #fn_name #generics #where_clause #body
	})
}

/// The component's one `on_add`, see the [module](self) docs: each step is a
/// statement on `world.reborrow()`, and the hook is emitted only when there is
/// a generated step to run.
struct OnAdd {
	/// The author's own `on_add`, taken out of their `#[component(..)]`.
	author: Option<TokenStream>,
	/// Pushes the declaration of the params the action takes.
	declare_params: Option<TokenStream>,
	/// Pushes the declaration of the params a route's exchange adapter reads.
	declare_exchange: Option<TokenStream>,
	/// The provider-integrity guard: fail loudly if a colocated explicit action
	/// takes the slot this struct's `#[require]` provides.
	guard: Option<TokenStream>,
}

impl OnAdd {
	/// Whether any generated step runs, so the hook is emitted.
	fn is_generated(&self) -> bool {
		self.declare_params.is_some()
			|| self.declare_exchange.is_some()
			|| self.guard.is_some()
	}

	/// Take the author's `on_add` out of `attrs` to run it first, when the hook
	/// is emitted; otherwise the author's attribute stays where it is.
	fn take_author(mut self, attrs: &mut Vec<syn::Attribute>) -> Self {
		if self.is_generated() {
			self.author = take_on_add(attrs)
				.map(|author| quote! { (#author)(world.reborrow(), cx); });
		}
		self
	}

	/// The `#[component(on_add = ..)]` attribute naming the generated fn.
	fn attr(&self) -> TokenStream {
		match self.is_generated() {
			true => quote! { #[component(on_add = Self::__action_on_add)] },
			false => TokenStream::default(),
		}
	}

	/// The generated fn, an associated fn so `Self` and the action's type
	/// params stay in scope for every step.
	fn make_fn(
		&self,
		fn_name: &syn::Ident,
		generics: &syn::Generics,
		bevy: &syn::Path,
	) -> Option<TokenStream> {
		if !self.is_generated() {
			return None;
		}
		let Self {
			author,
			declare_params,
			declare_exchange,
			guard,
		} = self;
		let (impl_generics, ty_generics, where_clause) =
			generics.split_for_impl();
		quote! {
			impl #impl_generics #fn_name #ty_generics #where_clause {
				#[doc(hidden)]
				#[allow(unused_mut)]
				fn __action_on_add(
					mut world: #bevy::ecs::world::DeferredWorld,
					cx: #bevy::ecs::lifecycle::HookContext,
				) {
					#author
					#declare_params
					#declare_exchange
					#guard
				}
			}
		}
		.into()
	}
}

/// Strip an `on_add = expr` out of a `#[component(..)]` attribute, returning the
/// expression so the generated hook runs it first. Other keys (`immutable`,
/// `on_remove`, ..) stay where they are, and an attribute left empty is
/// dropped.
fn take_on_add(attrs: &mut Vec<syn::Attribute>) -> Option<TokenStream> {
	let mut taken = None;
	attrs.retain_mut(|attr| {
		if taken.is_some() || !attr.path().is_ident("component") {
			return true;
		}
		// parsed as metas, so a generic path's commas stay in its expression
		let Ok(metas) = attr.parse_args_with(
			syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
		) else {
			return true;
		};
		let (on_add, kept): (Vec<_>, Vec<_>) = metas
			.into_iter()
			.partition(|meta| meta.path().is_ident("on_add"));
		let Some(syn::Meta::NameValue(on_add)) = on_add.into_iter().next()
		else {
			return true;
		};
		let value = on_add.value;
		taken = Some(quote! { #value });
		if kept.is_empty() {
			return false;
		}
		*attr = syn::parse_quote! { #[component(#(#kept),*)] };
		true
	});
	taken
}

/// The `_marker` field pinning a generic action's type params, `None` when the
/// action is not generic.
///
/// `pub` so a `<Foo field=x/>` struct-literal patch (which spreads
/// `..Default::default()`) resolves across module boundaries.
fn marker_field(
	generics: &syn::Generics,
	fn_attrs: &[syn::Attribute],
) -> Option<TokenStream> {
	let type_params: Vec<&syn::Ident> =
		generics.type_params().map(|tp| &tp.ident).collect();
	if type_params.is_empty() {
		return None;
	}
	let phantom = if type_params.len() == 1 {
		let tp = type_params[0];
		quote! { fn() -> #tp }
	} else {
		quote! { fn() -> (#(#type_params),*) }
	};
	let reflect_ignore = has_derive(fn_attrs, "Reflect")
		.then(|| quote! { #[reflect(ignore)] })
		.unwrap_or_default();
	Some(quote! {
		#reflect_ignore
		#[doc(hidden)]
		pub _marker: ::core::marker::PhantomData<#phantom>
	})
}

/// The struct's field initializers, given one expression per declared field.
fn struct_init(
	generics: &syn::Generics,
	fields: &[Prop],
	values: impl Iterator<Item = TokenStream>,
) -> TokenStream {
	let idents = fields.iter().map(|field| &field.ident);
	let marker = (generics.type_params().count() > 0)
		.then(|| quote! { _marker: ::core::marker::PhantomData, });
	if fields.is_empty() && marker.is_none() {
		quote! { Self }
	} else {
		quote! { Self { #(#idents: #values,)* #marker } }
	}
}

/// A perfect-derive `Clone`: cloning every field without bounding the
/// action's type params, which a `#[derive(Clone)]` would.
fn make_clone(
	fn_name: &syn::Ident,
	generics: &syn::Generics,
	fields: &[Prop],
) -> TokenStream {
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
	let values = fields.iter().map(|field| {
		let ident = &field.ident;
		quote! { ::core::clone::Clone::clone(&self.#ident) }
	});
	let init = struct_init(generics, fields, values);
	quote! {
		impl #impl_generics ::core::clone::Clone for #fn_name #ty_generics #where_clause {
			fn clone(&self) -> Self { #init }
		}
	}
}

/// A manual `Default`, honoring every `#[field(default = expr)]` and never
/// bounding the action's type params.
fn make_default(
	fn_name: &syn::Ident,
	generics: &syn::Generics,
	fields: &[Prop],
) -> TokenStream {
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
	let init =
		struct_init(generics, fields, fields.iter().map(Prop::default_value));
	quote! {
		impl #impl_generics ::core::default::Default for #fn_name #ty_generics #where_clause {
			fn default() -> Self { #init }
		}
	}
}

// ---------------------------------------------------------------------------
// Action-specific helpers
// ---------------------------------------------------------------------------

/// Convert a function/struct name to a snake_case standalone function
/// name with a `_action` suffix.
fn action_fn_name(fn_name: &syn::Ident) -> syn::Ident {
	let snake = fn_name.to_string().to_snake_case();
	let name = alloc::format!("{}_action", snake);
	syn::Ident::new(&name, fn_name.span())
}

/// Check whether any `#[derive(...)]` attribute contains `name`.
fn has_derive(attrs: &[syn::Attribute], name: &str) -> bool {
	attrs.iter().any(|attr| {
		if attr.path().is_ident("derive") {
			attr.parse_args_with(
				syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
			)
			.map(|meta_list| meta_list.iter().any(|path| path.is_ident(name)))
			.unwrap_or(false)
		} else {
			false
		}
	})
}

/// Generate turbofish syntax for generic type parameters,
/// or empty tokens when there are none.
fn make_turbofish(generics: &syn::Generics) -> TokenStream {
	let type_params: Vec<&syn::Ident> =
		generics.type_params().map(|tp| &tp.ident).collect();
	if type_params.is_empty() {
		TokenStream::default()
	} else {
		quote! { ::<#(#type_params),*> }
	}
}
// ===========================================================================
// Tests
// ===========================================================================

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

	// -----------------------------------------------------------------------
	// Action kind inference
	// -----------------------------------------------------------------------

	#[test]
	fn async_fn_uses_new_async() {
		let result = parse_str(
			quote!(),
			syn::parse_quote! { async fn my_action() -> i32 { 42 } },
		);
		assert!(result.contains("Action :: new_async"));
		assert!(result.contains("async fn my_action_action"));
	}

	#[test]
	fn async_local_uses_new_async_local() {
		let result = parse_str(
			quote!(local),
			syn::parse_quote! { async fn my_action() -> i32 { 42 } },
		);
		assert!(result.contains("Action :: new_async_local"));
		assert!(result.contains("async fn my_action_action"));
	}

	#[test]
	fn local_without_async_errors() {
		let err = parse_err(
			quote!(local),
			syn::parse_quote! { fn my_action() -> i32 { 42 } },
		);
		assert!(err.contains("only valid on async"));
	}

	#[test]
	fn pure_fn_uses_new_pure() {
		let result = parse_str(
			quote!(pure),
			syn::parse_quote! { fn my_action() -> i32 { 42 } },
		);
		assert!(result.contains("Action :: new_pure"));
		assert!(!result.contains("Action :: new_system"));
		assert!(!result.contains("Action :: new_async"));
	}

	#[test]
	fn plain_fn_uses_new_system() {
		let result = parse_str(
			quote!(),
			syn::parse_quote! { fn my_action(val: In<i32>) -> i32 { val.0 } },
		);
		assert!(result.contains("Action :: new_system"));
		assert!(!result.contains("Action :: new_pure"));
		assert!(!result.contains("Action :: new_async"));
	}

	// -----------------------------------------------------------------------
	// Multi-argument rejection
	// -----------------------------------------------------------------------

	#[test]
	fn multi_arg_async_errors() {
		let err = parse_err(
			quote!(),
			syn::parse_quote! { async fn bad(a: i32, b: i32) -> i32 { a + b } },
		);
		assert!(err.contains("at most one parameter"));
	}

	#[test]
	fn multi_arg_pure_errors() {
		let err = parse_err(
			quote!(pure),
			syn::parse_quote! { fn bad(a: i32, b: i32) -> i32 { a + b } },
		);
		assert!(err.contains("at most one parameter"));
	}

	// -----------------------------------------------------------------------
	// Pure (func) action tests
	// -----------------------------------------------------------------------

	#[test]
	fn pure_no_args_no_return() {
		let result =
			parse_str(quote!(pure), syn::parse_quote! { fn my_action() {} });
		assert!(result.contains("struct my_action"));
		assert!(result.contains("type In = ()"));
		assert!(result.contains("type Out = ()"));
		assert!(result.contains("let _ = __action_cx . input"));
		assert!(result.contains("Action :: new_pure"));
		assert!(result.contains("fn my_action_action"));
	}

	#[test]
	fn pure_single_arg() {
		let result = parse_str(
			quote!(pure),
			syn::parse_quote! { fn double(val: i32) -> i32 { val * 2 } },
		);
		assert!(result.contains("type In = i32"));
		assert!(result.contains("type Out = i32"));
		assert!(result.contains("let val = __action_cx . input"));
		assert!(result.contains("Action :: new_pure"));
		assert!(result.contains("fn double_action"));
	}

	#[test]
	fn pure_tuple_destructure() {
		let result = parse_str(
			quote!(pure),
			syn::parse_quote! { fn add((a, b): (i32, i32)) -> i32 { a + b } },
		);
		assert!(result.contains("type In = (i32 , i32)"));
		assert!(result.contains("type Out = i32"));
		assert!(result.contains("let (a , b) = __action_cx . input"));
		assert!(result.contains("Action :: new_pure"));
	}

	#[test]
	fn pure_result_return() {
		let result = parse_str(quote!(pure), syn::parse_quote! {
			fn fallible(val: i32) -> Result<i32> { Ok(val) }
		});
		assert!(result.contains("type Out = i32"));
		assert!(result.contains("Action :: new_pure"));
	}

	#[test]
	fn pure_result_out_flag() {
		let result = parse_str(quote!(pure, result_out), syn::parse_quote! {
			fn fallible(val: i32) -> Result<i32> { Ok(val) }
		});
		assert!(result.contains("type Out = Result < i32 >"));
		assert!(result.contains("Action :: new_pure"));
	}

	#[test]
	fn pure_passthrough_action_context() {
		let result = parse_str(quote!(pure), syn::parse_quote! {
			fn my_action(cx: ActionContext<i32>) -> i32 { *cx }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("Action :: new_pure"));
		assert!(result.contains("fn my_action_action"));
		assert!(result.contains("cx : "));
		assert!(!result.contains("__action_cx"));
	}

	#[test]
	fn pure_passthrough_default_unit() {
		let result = parse_str(quote!(pure), syn::parse_quote! {
			fn my_action(cx: ActionContext) -> i32 { 1 }
		});
		assert!(result.contains("type In = ()"));
		assert!(result.contains("Action :: new_pure"));
		assert!(!result.contains("__action_cx"));
	}

	#[test]
	fn visibility_preserved() {
		let result = parse_str(
			quote!(pure),
			syn::parse_quote! { pub fn public_action() {} },
		);
		assert!(result.contains("pub struct public_action"));
	}

	// -----------------------------------------------------------------------
	// Async action tests
	// -----------------------------------------------------------------------

	#[test]
	fn async_no_args() {
		let result = parse_str(
			quote!(),
			syn::parse_quote! { async fn my_action() -> i32 { 42 } },
		);
		assert!(result.contains("struct my_action"));
		assert!(result.contains("type In = ()"));
		assert!(result.contains("type Out = i32"));
		assert!(result.contains("Action :: new_async"));
		assert!(result.contains("async fn my_action_action"));
	}

	#[test]
	fn async_single_arg() {
		let result = parse_str(quote!(), syn::parse_quote! {
			async fn negate(val: i32) -> i32 { -val }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("Action :: new_async"));
		assert!(result.contains("let val = __action_cx . input"));
	}

	#[test]
	fn async_result_return() {
		let result = parse_str(quote!(), syn::parse_quote! {
			async fn fallible(val: i32) -> Result<i32> { Ok(val) }
		});
		assert!(result.contains("type Out = i32"));
		assert!(result.contains("Action :: new_async"));
	}

	#[test]
	fn async_result_out() {
		let result = parse_str(quote!(result_out), syn::parse_quote! {
			async fn fallible(val: i32) -> Result<i32> { Ok(val) }
		});
		assert!(result.contains("type Out = Result < i32 >"));
		assert!(result.contains("Action :: new_async"));
	}

	// -----------------------------------------------------------------------
	// Async passthrough
	// -----------------------------------------------------------------------

	#[test]
	fn async_passthrough_action_context() {
		let result = parse_str(quote!(), syn::parse_quote! {
			async fn my_action(cx: ActionContext<i32>) -> i32 { *cx }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("Action :: new_async"));
		assert!(result.contains("async fn my_action_action"));
		assert!(!result.contains("__action_cx"));
	}

	#[test]
	fn async_passthrough_default_unit() {
		let result = parse_str(quote!(), syn::parse_quote! {
			async fn my_action(cx: ActionContext) -> i32 { 42 }
		});
		assert!(result.contains("type In = ()"));
		assert!(result.contains("Action :: new_async"));
		assert!(!result.contains("__action_cx"));
	}

	// -----------------------------------------------------------------------
	// System action tests
	// -----------------------------------------------------------------------

	#[test]
	fn system_basic() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_action(val: In<i32>) -> i32 { val.0 * 2 }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("type Out = i32"));
		assert!(result.contains("Action :: new_system"));
		assert!(result.contains("ActionContext"));
		assert!(result.contains("let val = In (__action_cx . input)"));
	}

	#[test]
	fn system_with_system_params() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_action(val: In<i32>, time: Res<Time>) -> f32 { val.0 as f32 }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("Action :: new_system"));
		assert!(result.contains("time : Res < Time >"));
		assert!(result.contains("let val = In (__action_cx . input)"));
	}

	#[test]
	fn system_result() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_action(val: In<i32>) -> Result<i32> { Ok(val.0) }
		});
		assert!(result.contains("type Out = i32"));
		assert!(result.contains("Action :: new_system"));
	}

	#[test]
	fn system_unit_in_unit_out() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_action(val: In<()>) {}
		});
		assert!(result.contains("type In = ()"));
		assert!(result.contains("type Out = ()"));
		assert!(result.contains("Action :: new_system"));
	}

	#[test]
	fn system_no_input_all_system_params() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_action(commands: Commands) {}
		});
		assert!(result.contains("type In = ()"));
		assert!(result.contains("Action :: new_system"));
		// commands is a system param, not consumed as input
		assert!(result.contains("commands : Commands"));
		assert!(result.contains("let _ = __action_cx . input"));
	}

	// -----------------------------------------------------------------------
	// System passthrough
	// -----------------------------------------------------------------------

	#[test]
	fn system_passthrough_in_action_context() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_action(cx: In<ActionContext<i32>>) -> Entity { cx.id() }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("Action :: new_system"));
		assert!(result.contains("In (cx)"));
		assert!(!result.contains("__action_cx"));
	}

	#[test]
	fn system_passthrough_with_params() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_action(cx: In<ActionContext<i32>>, time: Res<Time>) -> f32 { 0.0 }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("Action :: new_system"));
		assert!(result.contains("time : Res < Time >"));
		assert!(result.contains("In (cx)"));
	}

	#[test]
	fn system_passthrough_default_unit() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_action(cx: In<ActionContext>) -> Entity { cx.id() }
		});
		assert!(result.contains("type In = ()"));
		assert!(result.contains("Action :: new_system"));
		assert!(result.contains("In (cx)"));
		assert!(!result.contains("__action_cx"));
	}

	#[test]
	fn system_passthrough_direct() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_action(cx: ActionContext<i32>) -> Entity { cx.id() }
		});
		assert!(result.contains("type In = i32"));
		assert!(result.contains("Action :: new_system"));
		assert!(result.contains("In (cx)"));
		assert!(!result.contains("__action_cx"));
	}

	#[test]
	fn system_passthrough_direct_default_unit() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_action(cx: ActionContext) -> Entity { cx.id() }
		});
		assert!(result.contains("type In = ()"));
		assert!(result.contains("Action :: new_system"));
		assert!(result.contains("In (cx)"));
		assert!(!result.contains("__action_cx"));
	}

	// -----------------------------------------------------------------------
	// Generics propagation
	// -----------------------------------------------------------------------

	#[test]
	fn async_passthrough_with_generics() {
		let result = parse_str(quote!(), syn::parse_quote! {
			async fn my_action<T>(input: ActionContext) -> ()
			where
				T: Send + Sync,
			{}
		});
		assert!(result.contains("struct my_action"));
		assert!(result.contains("PhantomData"));
		assert!(result.contains("where T : Send + Sync"));
		assert!(result.contains("impl < T >"));
		assert!(
			result
				.contains("IntoAction < my_action < T > > for my_action < T >")
		);
	}

	#[test]
	fn pure_action_with_generics() {
		let result = parse_str(quote!(pure), syn::parse_quote! {
			fn my_action<T>(val: i32) -> i32
			where
				T: Clone,
			{ val }
		});
		assert!(result.contains("PhantomData"));
		assert!(result.contains("where T : Clone"));
		assert!(result.contains("impl < T >"));
		assert!(
			result
				.contains("IntoAction < my_action < T > > for my_action < T >")
		);
		assert!(result.contains("Action :: new_pure"));
	}

	#[test]
	fn multi_generic_struct() {
		let result = parse_str(quote!(), syn::parse_quote! {
			async fn my_action<A, B>(input: ActionContext) -> ()
			where
				A: Send,
				B: Sync,
			{}
		});
		assert!(result.contains("PhantomData < fn () -> (A , B) >"));
	}

	#[test]
	fn generic_with_reflect_uses_fn_phantom() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			#[reflect(Component)]
			async fn my_action<T>(input: ActionContext) -> ()
			where
				T: Send + Sync,
			{}
		});
		assert!(result.contains("reflect (ignore)"));
		assert!(result.contains("fn () ->"));
	}

	// -----------------------------------------------------------------------
	// has_derive helper
	// -----------------------------------------------------------------------

	#[test]
	fn detect_component_derive() {
		let item: syn::ItemFn = syn::parse_quote! {
			#[derive(Debug, Clone, Component, Reflect)]
			fn Add() {}
		};
		assert!(!item.attrs.is_empty());
		assert!(has_derive(&item.attrs, "Component"));
	}

	#[test]
	fn detect_no_component_derive() {
		let item: syn::ItemFn = syn::parse_quote! {
			#[derive(Debug, Clone)]
			fn Add() {}
		};
		assert!(!item.attrs.is_empty());
		assert!(!has_derive(&item.attrs, "Component"));
	}

	#[test]
	fn detect_no_derives_at_all() {
		let item: syn::ItemFn = syn::parse_quote! {
			fn Add() {}
		};
		assert!(item.attrs.is_empty());
		assert!(!has_derive(&item.attrs, "Component"));
	}

	// -----------------------------------------------------------------------
	// Component derive + #[require] generation
	// -----------------------------------------------------------------------

	#[test]
	fn component_derive_adds_require_pure() {
		let result = parse_str(quote!(pure), syn::parse_quote! {
			#[derive(Debug, Clone, Component, Reflect)]
			fn Add(val: i32) -> i32 { val }
		});
		assert!(
			result.contains("derive (Debug , Clone , Component , Reflect)")
		);
		assert!(result.contains("# [require"));
		assert!(result.contains("Action <"));
		assert!(result.contains("Action :: new_pure (add_action"));
		assert!(result.contains("struct Add"));
		assert!(result.contains("fn add_action"));
	}

	#[test]
	fn no_component_derive_no_require() {
		let result = parse_str(quote!(pure), syn::parse_quote! {
			#[derive(Debug, Clone)]
			fn my_action(val: i32) -> i32 { val }
		});
		assert!(result.contains("derive (Debug , Clone)"));
		assert!(!result.contains("# [require"));
		assert!(result.contains("fn my_action_action"));
	}

	#[test]
	fn no_derives_no_require() {
		let result = parse_str(quote!(pure), syn::parse_quote! {
			fn my_action(val: i32) -> i32 { val }
		});
		assert!(!result.contains("# [require"));
		assert!(result.contains("fn my_action_action"));
	}

	#[test]
	fn async_component_derive_adds_require() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Debug, Component, Reflect)]
			async fn HelpHandler(cx: ActionContext<Request>) -> Result<Outcome<Response, Request>> {
				todo!()
			}
		});
		assert!(result.contains("derive (Debug , Component , Reflect)"));
		assert!(result.contains("# [require"));
		assert!(result.contains("Action :: new_async (help_handler_action"));
		assert!(result.contains("struct HelpHandler"));
		assert!(result.contains("async fn help_handler_action"));
	}

	#[test]
	fn system_component_derive_adds_require() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Debug, Component, Reflect)]
			fn Increment(val: In<()>, query: Query<&Name>) -> Result<i64> {
				Ok(1)
			}
		});
		assert!(result.contains("derive (Debug , Component , Reflect)"));
		assert!(result.contains("# [require"));
		assert!(result.contains("Action :: new_system (increment_action"));
		assert!(result.contains("struct Increment"));
		assert!(result.contains("fn increment_action"));
	}

	// -----------------------------------------------------------------------
	// ActionMeta require
	// -----------------------------------------------------------------------

	/// The macro cannot test a trait bound, so it probes each of `Self`, `In`
	/// and `Out` and takes whatever reflection they turn out to support —
	/// rather than making the author declare which tier applies.
	#[test]
	fn meta_probes_each_type() {
		let result = parse_str(quote!(pure), syn::parse_quote! {
			#[derive(Component, Reflect)]
			fn Add(val: i32) -> i32 { val }
		});
		assert!(result.contains("ActionMeta :: of :: < Add , i32 , i32 > ()"));
		assert!(result.contains("with_type_info"));
		assert!(result.contains("MaybeTyped :: < Add > :: new ()"));
		assert!(result.contains("MaybeTyped :: < i32 > :: new ()"));
		// the double autoref is what lets the `Typed` arm win when it applies
		assert!(result.contains("maybe_type_info"));
	}

	#[test]
	fn no_component_no_action_meta() {
		let result = parse_str(quote!(), syn::parse_quote! {
			fn my_action(val: In<i32>) -> i32 { val.0 }
		});
		assert!(!result.contains("ActionMeta"));
		assert!(!result.contains("MaybeTyped"));
	}

	/// Middleware wraps another entity's action rather than being one, and several
	/// share a host, so it claims no action slot: no `Action` require, no meta, no
	/// provider guard. Its `MiddlewareList::on_add` hook pushes `into_action()` onto the
	/// host's `MiddlewareList` instead.
	#[test]
	fn middleware_claims_no_action_slot() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Default, Clone, Component, Reflect)]
			async fn HelpHandler(cx: ActionContext<(Request, Next<Request, Response>)>) -> Result<Response> {
				todo!()
			}
		});
		assert!(!result.contains("# [require"));
		assert!(!result.contains("ActionMeta"));
		assert!(!result.contains("assert_provider"));
		assert!(result.contains("fn into_action"));
	}

	/// An action whose input does not reflect still owns its slot, and still
	/// gets whatever metadata its own type supports.
	#[test]
	fn unreflectable_input_still_owns_its_slot() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Default, Clone, Component, Reflect)]
			async fn EchoParams(cx: ActionContext<RequestParts>) -> MediaBytes {
				todo!()
			}
		});
		assert!(result.contains("# [require"));
		assert!(result.contains("MaybeTyped :: < RequestParts > :: new ()"));
	}

	// -----------------------------------------------------------------------
	// route attribute
	// -----------------------------------------------------------------------

	#[test]
	fn route_bare_adds_overload() {
		let result = parse_str(quote!(route), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn MyAction(val: i32) -> String { val.to_string() }
		});
		assert!(result.contains("ExchangeOverload"));
		assert!(result.contains("route :: exchange_overload ::"));
		assert!(result.contains("Action <"));
		assert!(!result.contains("PathPartial"));
	}

	#[test]
	fn route_with_path_adds_path_partial() {
		let result = parse_str(quote!(route = "home"), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn MyAction(val: i32) -> String { val.to_string() }
		});
		assert!(result.contains("ExchangeOverload"));
		assert!(result.contains("Action <"));
		assert!(result.contains("PathPartial"));
		assert!(result.contains("PathPartial :: new (\"home\")"));
	}

	#[test]
	fn route_with_expression_adds_path_partial() {
		let result =
			parse_str(quote!(route = get_route_path()), syn::parse_quote! {
				#[derive(Component, Reflect)]
				async fn MyAction(val: i32) -> String { val.to_string() }
			});
		assert!(result.contains("ExchangeOverload"));
		assert!(result.contains("PathPartial"));
		assert!(result.contains("PathPartial :: new (get_route_path ())"));
	}

	#[test]
	fn route_without_component_no_path_partial() {
		let result = parse_str(quote!(route = "home"), syn::parse_quote! {
			async fn my_action(val: i32) -> String { val.to_string() }
		});
		assert!(!result.contains("PathPartial"));
		assert!(!result.contains("# [require"));
	}

	#[test]
	fn route_with_path_and_action_meta() {
		let result = parse_str(quote!(route = "validate"), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn Validate(input: String) -> String { input }
		});
		assert!(result.contains("ExchangeOverload"));
		assert!(result.contains("PathPartial :: new (\"validate\")"));
		assert!(result.contains("with_type_info"));
	}

	// -----------------------------------------------------------------------
	// Doc and misc
	// -----------------------------------------------------------------------

	#[test]
	fn doc_attrs_forwarded() {
		let result = parse_str(quote!(pure), syn::parse_quote! {
			/// Does stuff.
			fn my_action(val: i32) -> i32 { val }
		});
		assert!(result.contains("doc"));
		assert!(result.contains("struct my_action"));
	}

	#[test]
	fn handler_at_module_level() {
		let result = parse_str(quote!(pure), syn::parse_quote! {
			fn add(val: i32) -> i32 { val }
		});
		// The handler function should be at module level so #[require] can reference it
		assert!(result.contains("fn into_action"));
		assert!(result.contains("Action :: new_pure (add_action)"));
		assert!(result.contains("fn add_action"));
	}

	#[test]
	fn generic_component_with_turbofish() {
		let result = parse_str(quote!(pure), syn::parse_quote! {
			#[derive(Debug, Component)]
			fn MyAction<T>(val: i32) -> i32
			where T: Clone,
			{ val }
		});
		assert!(result.contains("# [require"));
		assert!(
			result.contains("Action :: new_pure (my_action_action :: < T >")
		);
		assert!(result.contains("fn my_action_action < T >"));
	}

	#[test]
	fn no_default_suppresses_auto_default() {
		let result = parse_str(
			quote!(no_default),
			syn::parse_quote! { async fn my_action() -> i32 { 42 } },
		);
		assert!(!result.contains("Default for my_action"));
	}

	#[test]
	fn auto_default_for_unit_struct() {
		let result = parse_str(
			quote!(),
			syn::parse_quote! { async fn my_action() -> i32 { 42 } },
		);
		assert!(result.contains("Default for my_action"));
	}

	#[test]
	fn no_clone_suppresses_auto_clone() {
		let result = parse_str(
			quote!(no_clone),
			syn::parse_quote! { async fn my_action() -> i32 { 42 } },
		);
		assert!(!result.contains("derive (Clone)"));
	}

	#[test]
	fn user_derive_default_no_double_impl() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Default, Component)]
			async fn my_action() -> i32 { 42 }
		});
		// derive(Default) is in the user attrs, so macro should NOT generate impl Default
		assert!(!result.contains("Default for my_action"));
	}

	// -----------------------------------------------------------------------
	// `#[field]` actions
	// -----------------------------------------------------------------------

	#[test]
	fn field_becomes_a_pub_struct_field() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn RepeatTimes(#[field] total_times: u32, cx: ActionContext) -> Result<Outcome> {
				todo!()
			}
		});
		assert!(result.contains("pub total_times : u32"));
		// the handler takes the field by value, ahead of the input
		assert!(result.contains(
			"async fn repeat_times_action (total_times : u32 , cx :"
		));
		// the require site installs the live-fetching wrapper, not the handler
		assert!(
			result.contains("Action :: new_async (repeat_times_action_fields)")
		);
		assert!(result.contains("get :: < RepeatTimes , _ >"));
	}

	#[test]
	fn field_requires_a_component_derive() {
		let err = parse_err(quote!(), syn::parse_quote! {
			async fn Bare(#[field] total: u32) -> Result<Outcome> { todo!() }
		});
		assert!(err.contains("requires `#[derive(Component)]`"));
	}

	#[test]
	fn generic_field_struct_keeps_a_named_marker() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn RepeatTimes<Input>(
				#[field] total_times: u32,
				cx: ActionContext<Input>,
			) -> Result<Outcome>
			where
				Input: 'static + Send + Sync + Clone,
			{ todo!() }
		});
		assert!(result.contains(
			"pub _marker : :: core :: marker :: PhantomData < fn () -> Input >"
		));
		assert!(result.contains("reflect (ignore)"));
		// the perfect-derive clone, so `Input` is never spuriously bounded
		assert!(result.contains("Clone for RepeatTimes < Input >"));
		assert!(!result.contains("derive (Clone)"));
	}

	#[test]
	fn optional_field_stores_as_declared() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn Log(#[field] message: Option<SmolStr>, cx: ActionContext) -> Result<Outcome> {
				todo!()
			}
		});
		assert!(result.contains("message : Option < SmolStr >"));
	}

	#[test]
	fn required_field_validates_by_name() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn RunNext(#[field(required)] target: Entity, cx: ActionContext) -> Result<Entity> {
				todo!()
			}
		});
		assert!(result.contains("Option < Entity >"));
		assert!(result.contains("missing required field"));
		assert!(result.contains("\"target\""));
	}

	#[test]
	fn default_expr_forces_a_manual_default() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn Keep(#[field(default = 2)] keep: usize, cx: ActionContext) -> Result<Outcome> {
				todo!()
			}
		});
		assert!(result.contains("Default for Keep"));
		assert!(result.contains("keep : (2) . into ()"));
	}

	#[test]
	fn default_expr_conflicts_with_derived_default() {
		let err = parse_err(quote!(), syn::parse_quote! {
			#[derive(Default, Component, Reflect)]
			async fn Keep(#[field(default = 2)] keep: usize, cx: ActionContext) -> Result<Outcome> {
				todo!()
			}
		});
		assert!(err.contains("conflicts with `#[derive(Default)]`"));
	}

	#[test]
	fn mut_field_binds_mutably_through_a_mut_query() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			fn SucceedTimes(
				#[field] max_times: u32,
				#[field(mut)] times: u32,
				cx: In<ActionContext>,
			) -> Result<Outcome> { todo!() }
		});
		assert!(result.contains("Query < & mut SucceedTimes >"));
		assert!(result.contains("get_mut (__action_cx . id ())"));
		assert!(result.contains("let times = & mut __action_item . times"));
		assert!(result.contains("times : & mut u32"));
		// nothing to write back to in a detached action, so no `IntoAction`
		assert!(!result.contains("fn into_action"));
	}

	#[test]
	fn mut_field_on_async_errors() {
		let err = parse_err(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn Bad(#[field(mut)] times: u32, cx: ActionContext) -> Result<Outcome> {
				todo!()
			}
		});
		assert!(err.contains("not valid on an async action"));
	}

	#[test]
	fn mut_field_cannot_be_required() {
		let err = parse_err(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			fn Bad(#[field(mut, required)] times: u32, cx: In<ActionContext>) -> Result<Outcome> {
				todo!()
			}
		});
		assert!(err.contains("cannot be combined with"));
	}

	/// A pure body cannot reach the world, so a pure action carrying fields
	/// installs a system wrapper while keeping its pure `into_action`.
	#[test]
	fn pure_fields_lower_to_a_system() {
		let result = parse_str(quote!(pure), syn::parse_quote! {
			#[derive(Component, Reflect)]
			fn Add(#[field] rhs: i32, val: i32) -> i32 { val + rhs }
		});
		assert!(result.contains("Action :: new_system (add_action_fields)"));
		assert!(result.contains("Query < & Add >"));
		// the handler itself stays pure, and so does the detached action
		assert!(result.contains("fn add_action (rhs : i32 , __action_cx :"));
		assert!(result.contains("Action :: new_pure (move |"));
	}

	/// A detached action has no component to read, so an async field action
	/// freezes its fields at conversion.
	#[test]
	fn into_action_captures_the_fields() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn Greet(#[field] name: String, cx: ActionContext) -> Result<Outcome> {
				todo!()
			}
		});
		assert!(result.contains("let __action_fields = self"));
		assert!(result.contains("let Greet { name , .. } = __action_fields"));
	}

	/// Middleware's component lives on the live host entity the call names as
	/// caller, so it keeps the live-fetching wrapper rather than freezing.
	#[test]
	fn middleware_fields_stay_live() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn Policy(
				#[field] label: String,
				cx: ActionContext<(Request, Next<Request, Response>)>,
			) -> Result<Response> { todo!() }
		});
		assert!(result.contains("Action :: new_async (policy_action_fields)"));
		assert!(!result.contains("let __action_fields = self"));
	}

	/// A system field action forwards the author's params, keeping a `#[cfg]`
	/// gate on both the declaration and the call.
	#[test]
	fn system_fields_forward_params() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			fn Tally(
				#[field] scale: u32,
				cx: In<ActionContext>,
				#[cfg(feature = "tui")] sessions: Query<&Name>,
			) -> Result<Outcome> { todo!() }
		});
		assert!(
			result.contains(
				"cfg (feature = \"tui\")] sessions : Query < & Name >"
			)
		);
		assert!(result.contains("cfg (feature = \"tui\")] sessions)"));
	}

	/// Bevy keeps one `on_add` per component and silently takes the last, so the
	/// author's hook runs first inside the one generated hook.
	#[test]
	fn author_on_add_runs_inside_the_generated_hook() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			#[component(on_add = hook_ext::entity_hook(FixedPage::insert_page_root))]
			async fn FixedPage(cx: ActionContext<Request>) -> Result<PageRequest> {
				todo!()
			}
		});
		assert!(result.contains("on_add = Self :: __action_on_add"));
		assert!(result.contains("assert_provider :: < Self >"));
		// exactly one `on_add`, so neither hook is dropped
		assert_eq!(result.matches("on_add =").count(), 1);
	}

	/// The generated hook runs the author's hook, the params declaration, the
	/// exchange declaration and the guard, in that order.
	#[test]
	fn generated_on_add_runs_in_order() {
		let result = parse_str(quote!(route), syn::parse_quote! {
			#[derive(Component, Reflect)]
			#[component(on_add = author_hook)]
			async fn Serve(
				#[param] port: u16,
				cx: ActionContext<RequestParts>,
			) -> Result<String> {
				todo!()
			}
		});
		let steps = [
			"(author_hook) (world . reborrow () , cx)",
			"RouteParams :: declare",
			"route :: declare_exchange",
			"assert_provider",
		]
		.map(|step| result.find(step).unwrap());
		assert!(steps.windows(2).all(|pair| pair[0] < pair[1]));
	}

	/// An inline `#[param]` lands on a hidden params struct carrying its doc
	/// and short flag, declared at route reach and read before the body.
	#[test]
	fn inline_param_declares_and_reads() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn Audit(
				/// Repair what the audit finds.
				#[param(short = 'f')] mut fix: bool,
				cx: ActionContext<Request>,
			) -> Result<String> {
				todo!()
			}
		});
		assert!(result.contains("struct AuditParams"));
		assert!(result.contains("ParamOptions :: short ('f')"));
		assert!(result.contains("Repair what the audit finds."));
		assert!(
			result.contains(
				"RouteParams :: declare :: < Self , (AuditParams ,) >"
			)
		);
		assert!(result.contains(
			"let AuditParams { mut fix , } = beet_net :: prelude :: RequestPartsRef :: request_parts (& cx . input) . parse_params :: < AuditParams > () ?"
		));
		// the param is no handler argument
		assert!(result.contains("async fn audit_action (cx :"));
	}

	/// A `Params<T>` group is declared and read whole, with no hidden struct.
	#[test]
	fn params_group_declares_and_reads() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn Run(
				entry: Params<EntryParams>,
				cx: ActionContext<Request>,
			) -> Result<String> {
				todo!()
			}
		});
		assert!(!result.contains("struct RunParams"));
		assert!(
			result.contains(
				"RouteParams :: declare :: < Self , (EntryParams ,) >"
			)
		);
		assert!(result.contains(
			"let entry = beet_net :: prelude :: Params (beet_net :: prelude :: RequestPartsRef :: request_parts (& cx . input) . parse_params :: < EntryParams > () ?)"
		));
	}

	/// `AnyParams` binds every param the request carries and declares the
	/// route as taking any.
	#[test]
	fn any_params_binds_every_param() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn RunWasm(any: AnyParams, cx: ActionContext<Request>) -> Result<String> {
				todo!()
			}
		});
		assert!(result.contains("(beet_net :: prelude :: AnyParams ,)"));
		assert!(result.contains(
			"let any = beet_net :: prelude :: AnyParams :: from (beet_net :: prelude :: RequestPartsRef :: request_parts (& cx . input))"
		));
	}

	/// An action taking params with no input of its own reads them off the
	/// request parts, which need no body.
	#[test]
	fn params_infer_a_request_input() {
		for item in [
			syn::parse_quote! {
				#[derive(Component, Reflect)]
				async fn Sync(#[param] dry_run: bool) -> Result<String> { todo!() }
			},
			syn::parse_quote! {
				#[derive(Component, Reflect)]
				fn Count(#[param] dry_run: bool, names: Query<&Name>) -> Result<usize> { todo!() }
			},
		] {
			let result = parse_str(quote!(), item);
			assert!(
				result
					.contains("type In = beet_net :: prelude :: RequestParts")
			);
			assert!(result.contains("request_parts (& __action_cx . input)"));
		}
	}

	/// Middleware's params reach every route beneath it.
	#[test]
	fn middleware_params_reach_beneath() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn HelpHandler(
				#[param] help: bool,
				cx: ActionContext<(Request, Next<Request, Response>)>,
			) -> Result<Response> { todo!() }
		});
		assert!(result.contains("SubtreeParams :: declare"));
		assert!(!result.contains("RouteParams :: declare"));
	}

	/// A route declares what its exchange adapter's conversions read, beside the
	/// adapter it requires; one already answering a `Response` converts nothing.
	#[test]
	fn route_declares_its_exchange() {
		let result = parse_str(quote!(route), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn Page(cx: ActionContext<Request>) -> Result<PageRequest> {
				todo!()
			}
		});
		assert!(result.contains(
			"route :: exchange_overload :: < Request , PageRequest , _ , _ >"
		));
		assert!(result.contains(
			"route :: declare_exchange :: < Request , PageRequest , _ , _ >"
		));
		let result = parse_str(quote!(route), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn Echo(cx: ActionContext<Request>) -> Result<Response> {
				todo!()
			}
		});
		assert!(!result.contains("declare_exchange"));
	}

	/// A `Request -> Response` route needs no adapter, so the attribute names no
	/// beet_router type and serves a crate beneath it.
	#[test]
	fn request_response_route_needs_no_adapter() {
		let result =
			parse_str(quote!(route = "get/:name"), syn::parse_quote! {
				#[derive(Component, Reflect)]
				async fn Get(cx: ActionContext<Request>) -> Result<Response> { todo!() }
			});
		assert!(result.contains("PathPartial :: new (\"get/:name\")"));
		assert!(!result.contains("ExchangeOverload"));
	}

	/// A bad param is the caller's `400`, which needs a `Result` to raise.
	#[test]
	fn params_require_a_result() {
		let err = parse_err(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn Bad(#[param] fix: bool, cx: ActionContext<Request>) -> String {
				todo!()
			}
		});
		assert!(err.contains("an action taking params returns `Result`"));
	}

	/// A generic action keeps its type params in the generated hook, so it may
	/// take params like any other.
	#[test]
	fn generic_action_takes_params() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn Typed<T>(
				#[param] verbose: bool,
				cx: ActionContext<Request>,
			) -> Result<String>
			where
				T: 'static + Send + Sync,
			{
				todo!()
			}
		});
		assert!(result.contains(
			"impl < T > Typed < T > where T : 'static + Send + Sync"
		));
		assert!(result.contains("fn __action_on_add"));
	}

	/// A `no_clone` field lands on the struct but is never bound, so an
	/// expensive value is read through the caller instead of cloned per call.
	#[test]
	fn no_clone_field_is_unbound() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn Upload(
				#[field] name: String,
				#[field(no_clone)] bytes: Vec<u8>,
				cx: ActionContext,
			) -> Result<Outcome> { todo!() }
		});
		// on the struct
		assert!(result.contains("pub bytes : Vec < u8 >"));
		// but not in the handler, and not in the fetch
		assert!(
			result.contains("async fn upload_action (name : String , cx :")
		);
		assert!(!result.contains("__action_item . bytes"));
	}

	/// A field can narrow its visibility; the default stays `pub` so a
	/// cross-module `rsx!` struct-literal patch resolves.
	#[test]
	fn field_visibility_is_declarable() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			async fn Secretive(
				#[field(pub(crate))] token: String,
				cx: ActionContext,
			) -> Result<Outcome> { todo!() }
		});
		assert!(result.contains("pub (crate) token : String"));
	}

	/// A system action freezes its fields too: bevy refuses a non-ZST cached
	/// system, so they ride in as input rather than in a closure.
	#[test]
	fn system_into_action_threads_fields_as_input() {
		let result = parse_str(quote!(), syn::parse_quote! {
			#[derive(Component, Reflect)]
			fn Tally(#[field] scale: u32, cx: In<ActionContext>) -> Result<Outcome> {
				todo!()
			}
		});
		assert!(
			result.contains("new_system_with (self , tally_action_captured)")
		);
		assert!(result.contains("In < (Tally ,"));
	}
}

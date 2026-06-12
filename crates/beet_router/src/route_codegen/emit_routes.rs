//! Emits the per-collection route bundle function.
//!
//! Each collection emits a `pub fn <name>_routes() -> impl Bundle` that spawns
//! one child per route via [`spawn_with`], so multiple collections can be merged
//! onto a single router entity (eg passed together to `default_router`) without
//! clobbering each other's children.

use crate::prelude::*;
use crate::route_codegen::syn_utils::action_input_ty;
use crate::route_codegen::syn_utils::action_output_ty;
use crate::route_codegen::syn_utils::type_last_ident;
use beet_core::prelude::*;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::ItemFn;

/// Classification of a handler function signature, mirroring the `#[action]`
/// macro and the runtime `render_action::*_route` constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandlerKind {
	/// `fn get() -> impl Bundle` — no context, rendered once.
	Static,
	/// `fn get(cx: ActionContext<In>) -> ..`
	Pure,
	/// `async fn get(cx: ActionContext<In>) -> ..`
	Async,
	/// `fn get(cx: In<ActionContext<In>>, ..) -> ..`
	System,
}

impl HandlerKind {
	/// Classifies a handler from its parsed signature.
	fn of(item: &ItemFn) -> Self {
		if item.sig.asyncness.is_some() {
			return Self::Async;
		}
		if item.sig.inputs.is_empty() {
			return Self::Static;
		}
		match first_param_outer_ident(item).as_deref() {
			Some("In") => Self::System,
			_ => Self::Pure,
		}
	}
}

/// Returns the identifier of the first parameter's outermost type path segment,
/// ie `In` for `In<ActionContext<T>>` or `ActionContext` for `ActionContext<T>`.
fn first_param_outer_ident(item: &ItemFn) -> Option<String> {
	let syn::FnArg::Typed(pat_type) = item.sig.inputs.first()? else {
		return None;
	};
	let syn::Type::Path(type_path) = &*pat_type.ty else {
		return None;
	};
	type_path
		.path
		.segments
		.last()
		.map(|seg| seg.ident.to_string())
}

/// Adds the module declarations and the collection bundle function to the
/// collection's codegen file.
pub(crate) fn emit_collection(
	collection: &RouteCollection,
	files: &[RouteFile],
) -> Result<CodegenFile> {
	let mut codegen = collection
		.codegen
		.clone_info(collection.codegen.output().clone());

	let mut children = Vec::<TokenStream>::new();

	for file in files {
		match &file.kind {
			RouteFileKind::Rust {
				mod_ident,
				mod_path,
				methods,
			} => {
				codegen.add_item(item_mod(mod_ident, mod_path, collection));
				for method in methods {
					children.push(emit_rust_route(
						collection, file, mod_ident, method,
					)?);
				}
			}
			RouteFileKind::Blob { store_path, meta } => {
				children.push(emit_blob_route(file, store_path, meta));
			}
		}
	}

	let collection_ident = format_ident!("{}_routes", collection.name()?);
	// A uniquely-typed `OnSpawnTyped` effect that appends each route as a child.
	// Multiple collections compose onto one router entity this way without
	// clobbering each other, and without the duplicate-`Children` error that a
	// shared `SpawnRelated` bundle would cause.
	let body = quote! {
		OnSpawnTyped::new(move |parent: &mut EntityWorldMut| {
			let parent_id = parent.id();
			parent.world_scope(move |world| {
				#(world.spawn((ChildOf(parent_id), #children));)*
			});
		})
	};
	let cfg = server_cfg(collection);
	let item: ItemFn = syn::parse_quote! {
		#cfg
		pub fn #collection_ident() -> impl Bundle {
			#body
		}
	};
	codegen.add_item(item);
	Ok(codegen)
}

/// Emits the route bundle for a single Rust handler.
fn emit_rust_route(
	collection: &RouteCollection,
	file: &RouteFile,
	mod_ident: &syn::Ident,
	method: &RouteMethod,
) -> Result<TokenStream> {
	let func = &method.item.sig.ident;
	let path = file.route_path.to_string();
	let http = method.method.self_token_stream();
	let kind = HandlerKind::of(&method.item);

	match collection.category {
		RouteCollectionCategory::Pages => {
			// Page handlers return `impl Bundle` (an `rsx!` tree); the route
			// family builds them through the template substrate per request.
			let route = match kind {
				HandlerKind::Static => {
					quote! { render_action::fixed_func_route(#path, #mod_ident::#func) }
				}
				HandlerKind::Pure => {
					quote! { render_action::pure_route(#path, #mod_ident::#func) }
				}
				HandlerKind::Async => {
					quote! { render_action::async_route(#path, #mod_ident::#func) }
				}
				HandlerKind::System => {
					quote! { render_action::system_route(#path, #mod_ident::#func) }
				}
			};
			let cache = match kind {
				HandlerKind::Static => quote! { CacheStrategy::Static },
				_ => quote! { CacheStrategy::Dynamic },
			};
			Ok(quote! { (#route, #http, #cache) })
		}
		RouteCollectionCategory::Actions => {
			// Pin In/Out so `Result<T>` unwraps to `T` (otherwise `IntoResult`
			// is ambiguous between the identity and `Result` impls).
			let in_ty = action_input_ty(&method.item)
				.unwrap_or_else(|| syn::parse_quote!(()));
			let out_ty = action_output_ty(&method.item);
			let ctor = match kind {
				HandlerKind::Async => {
					quote! { Action::<#in_ty, #out_ty>::new_async(#mod_ident::#func) }
				}
				HandlerKind::System => {
					quote! { Action::<#in_ty, #out_ty>::new_system(#mod_ident::#func) }
				}
				HandlerKind::Static | HandlerKind::Pure => {
					quote! { Action::<#in_ty, #out_ty>::new_pure(#mod_ident::#func) }
				}
			};
			// `Json<T>` implements `FromRequest` via both its dedicated impl
			// and the `DeserializeOwned` blanket, so pin the marker to the
			// dedicated `Json<T>` impl.
			let exchange = if type_last_ident(&in_ty).as_deref() == Some("Json")
			{
				quote! { exchange_route::<#in_ty, #out_ty, _, #in_ty, _, _>(#path, #ctor) }
			} else {
				quote! { exchange_route(#path, #ctor) }
			};
			Ok(quote! { (#exchange, #http, CacheStrategy::Dynamic) })
		}
	}
}

/// Builds the `#[path = ..] pub mod ident;` declaration for a handler file.
///
/// Actions collections gate the module behind the configured server feature so
/// the server-only handlers are not pulled into wasm client builds.
fn item_mod(
	mod_ident: &syn::Ident,
	mod_path: &str,
	collection: &RouteCollection,
) -> syn::ItemMod {
	let cfg = server_cfg(collection);
	syn::parse_quote! {
		#[path = #mod_path]
		#cfg
		pub mod #mod_ident;
	}
}

/// The `#[cfg(feature = ..)]` gate for an actions collection, if configured.
fn server_cfg(collection: &RouteCollection) -> Option<syn::Attribute> {
	match (collection.category, &collection.server_feature) {
		(RouteCollectionCategory::Actions, Some(feature)) => {
			Some(syn::parse_quote!(#[cfg(feature = #feature)]))
		}
		_ => None,
	}
}

/// Emits the route bundle for a markdown/html content file, including its
/// scan-time [`ArticleMeta`] when the frontmatter declared one.
fn emit_blob_route(
	file: &RouteFile,
	store_path: &SmolPath,
	meta: &Option<ArticleMeta>,
) -> TokenStream {
	let path = file.route_path.to_string();
	let store_path = store_path.to_string();
	let meta = meta.as_ref().map(|meta| {
		let meta = meta.self_token_stream();
		quote! { #meta, }
	});
	quote! {
		(
			route(#path, BlobScene::new(#store_path)),
			HttpMethod::Get,
			CacheStrategy::Static,
			#meta
		)
	}
}

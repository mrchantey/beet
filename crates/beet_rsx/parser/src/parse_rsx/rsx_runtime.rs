use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use syn::spanned::Spanned;

#[derive(Debug, Clone)]
pub struct RsxRuntime {
	/// the identifier that contains the effect registration functions,
	/// ie `Sigfault`, it will be called like `#effect::parse_block_node(#block)`
	pub effect: syn::Path,
	pub event: syn::Path,
}
impl Default for RsxRuntime {
	fn default() -> Self { Self::sigfault() }
}
impl RsxRuntime {
	pub fn sigfault() -> Self {
		Self {
			effect: syn::parse_quote!(beet::rsx::sigfault::Sigfault),
			event: syn::parse_quote!(beet::prelude::EventRegistry),
		}
	}
	pub fn bevy() -> Self {
		Self {
			effect: syn::parse_quote!(beet::rsx::bevy::BevyRuntime),
			event: syn::parse_quote!(beet::rsx::bevy::BevyEventRegistry),
		}
	}
	/// Updates [`Self::effect`] to the given runtime. Built-in runtimes
	/// have a shorthand:
	/// - `sigfault` -> `beet::rsx::sigfault::Sigfault`
	/// - `bevy` -> `beet::rsx::bevy::BevyRuntime`
	pub fn set(&mut self, runtime: &str) -> syn::Result<()> {
		*self = match runtime {
			"sigfault" => Self::sigfault(),
			"bevy" => Self::bevy(),
			_ => {
				let path: syn::Path = syn::parse_str(&runtime)?;
				Self {
					effect: path.clone(),
					event: path,
				}
			}
		};
		Ok(())
	}
	/// Create the tokens to register an event inside an effect,
	/// these tokens depend on a variable `loc` that is the location of the
	/// event in the tree.
	pub fn register_event_tokens(
		&self,
		key: &str,
		value: impl ToTokens + Spanned,
	) -> TokenStream {
		let register_func =
			syn::Ident::new(&format!("register_{key}"), value.span());
		let event_registry = &self.event;
		quote! {
			#event_registry::#register_func(#key,loc, #value);
		}
	}
}

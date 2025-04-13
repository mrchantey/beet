mod html_tokens_to_ron;
mod rsx_runtime;
pub use rsx_runtime::*;
pub use html_tokens_to_ron::*;
mod rsx_pipeline;
pub use rsx_pipeline::*;
mod html_tokens;
pub use html_tokens::*;
mod html_tokens_to_rust;
pub use html_tokens_to_rust::*;
mod rsx_node_tokens;
pub use rsx_node_tokens::*;
mod rstml_to_html_tokens;
pub use rstml_to_html_tokens::*;
mod meta_builder;
mod rstml_rust_to_hash;
mod rusty_tracker_builder;
#[cfg(feature = "html")]
mod string_to_html_tokens;
#[cfg(feature = "css")]
mod validate_css;
pub use meta_builder::*;
pub use rusty_tracker_builder::*;
#[cfg(feature = "html")]
pub use string_to_html_tokens::*;
#[cfg(feature = "css")]
pub use validate_css::*;
pub mod tokens_to_rstml;
pub use self::rstml_rust_to_hash::*;
pub use self::tokens_to_rstml::*;


#[derive(Debug, Clone)]
pub struct RsxIdents {
	pub mac: syn::Ident,
	pub runtime: RsxRuntime,
}
/// Get the default RsxIdents.
/// Usually implementers of [`beet_rsx_parser`] will have their
/// own mechanism for overriding defaults, ie [`beet_rsx_macros`] would use
/// feature flags and [`beet_cli`] would use cli args.
impl Default for RsxIdents {
	fn default() -> Self {
		Self {
			mac: syn::parse_quote!(rsx),
			runtime: RsxRuntime::default(),
		}
	}
}

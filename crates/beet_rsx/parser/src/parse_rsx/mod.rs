mod html_tokens_to_ron;
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
mod rstml_to_rsx_template;
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
#[allow(unused_imports)]
pub use self::rstml_to_rsx_template::*;
pub use self::tokens_to_rstml::*;
pub mod rsx_file_visitor;
#[allow(unused_imports)]
pub use self::rsx_file_visitor::*;
pub mod rstml_to_rsx_direct;
#[allow(unused_imports)]
pub use self::rstml_to_rsx_direct::*;
use proc_macro2::TokenStream;
use syn::Expr;
use syn::File;
use syn::visit_mut::VisitMut;
#[derive(Debug, Clone)]
pub struct RsxIdents {
    pub mac: syn::Ident,
    pub runtime: RsxRuntime,
}
#[derive(Debug, Clone)]
pub struct RsxRuntime {
    /// the identifier that contains the effect registration functions,
    /// ie `Sigfault`, it will be called like `#effect::parse_block_node(#block)`
    pub effect: syn::Path,
    pub event: syn::Path,
}
impl Default for RsxRuntime {
    fn default() -> Self {
        Self::sigfault()
    }
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
#[derive(Debug, Clone)]
pub struct ParseRsx {
    pub include_errors: bool,
    pub idents: RsxIdents,
}
impl Default for ParseRsx {
    fn default() -> Self {
        Self {
            include_errors: true,
            idents: Default::default(),
        }
    }
}
impl ParseRsx {
    /// header to add to the top of each rust file
    pub const SHEBANG: &'static str = "// 🥁 AUTOGENERATED BY BEET 🥁\n// 🥁 AUTOGENERATED BY BEET 🥁\n// 🥁 AUTOGENERATED BY BEET 🥁";
    /// currently unused
    /// entrypoint for file (preprosessor) parsing
    pub fn parse_file(&mut self, file: &str) -> syn::Result<(File, RsxFileVisitorOut)> {
        self.include_errors = false;
        let mut file = syn::parse_file(file)?;
        let mut visitor = RsxFileVisitor::new(self);
        visitor.visit_file_mut(&mut file);
        file.shebang = Some(Self::SHEBANG.to_string());
        Ok((file, visitor.into()))
    }
    /// entrypoint for inline (macro) parsing.
    /// Called when visiting an rsx macro.
    /// Mutated in place for efficient file parsing
    pub fn parse_rsx(&mut self, _tokens: &mut TokenStream) -> RstmlToRsx {
        todo!("use rstml_to_rsx");
    }
    /// Check if a path matches the macro, by default only the last segment is checked
    pub fn path_matches(&self, path: &syn::Path) -> bool {
        path.segments.last().map_or(false, |seg| seg.ident == self.idents.mac)
    }
}
pub fn macro_or_err(expr: &Expr) -> syn::Result<&syn::Macro> {
    if let Expr::Macro(mac) = expr {
        Ok(&mac.mac)
    } else {
        Err(syn::Error::new_spanned(expr, "expected macro"))
    }
}

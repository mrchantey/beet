use quote::format_ident;
use syn::Ident;



/// implemented by [`#derive(Node)`]
pub fn builder_ident(ident: &Ident) -> Ident {
	format_ident!("{}Builder", ident)
}
/// implemented by [`#derive(Node)`]
pub fn required_ident(ident: &Ident) -> Ident {
	format_ident!("{}Required", ident)
}

/// implemented by [`#derive(Buildable)`]
pub fn buildable_ident(ident: &Ident) -> Ident {
	format_ident!("{}Buildable", ident)
}

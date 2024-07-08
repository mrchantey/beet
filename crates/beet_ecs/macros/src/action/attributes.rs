use syn::Expr;
use crate::utils::punctuated_args;

pub struct ActionAttributes {
	pub observers_generic: Vec<Expr>,
	pub observers_non_generic: Vec<Expr>,
}


impl ActionAttributes {
	pub fn parse(attrs: &[syn::Attribute]) -> syn::Result<Self> {
		let mut observers_generic = Vec::new();
		let mut observers_non_generic = Vec::new();

		for attr in attrs {
			let attr_str =
				attr.path().get_ident().map(|ident| ident.to_string());
			match attr_str.as_ref().map(|a| a.as_str()) {
				Some("observers") => {
					observers_non_generic.extend(punctuated_args(attr.parse_args()?)?);
				}
				Some("generic_observers") => {
					observers_generic.extend(punctuated_args(attr.parse_args()?)?);
				}
				_ => {}
			}
		}
		return Ok(Self {
			observers_generic,
			observers_non_generic,
		});
	}
}

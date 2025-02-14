use crate::utils::punctuated_args;
use syn::Expr;

#[derive(Default)]
pub struct ActionAttributes {
	pub observers: Vec<Expr>,
	pub storage: Option<Expr>,
	pub require: Vec<Expr>,
}

impl ActionAttributes {
	pub fn parse(attrs: &[syn::Attribute]) -> syn::Result<Self> {
		let mut this = Self::default();

		for attr in attrs {
			let attr_str =
				attr.path().get_ident().map(|ident| ident.to_string());
			match attr_str.as_ref().map(|a| a.as_str()) {
				Some("observers") => {
					this.observers.extend(punctuated_args(attr.parse_args()?)?);
				}
				Some("require") => {
					this.require.extend(punctuated_args(attr.parse_args()?)?);
				}
				Some("storage") => {
					this.storage = Some(attr.parse_args()?);
				}
				_ => {}
			}
		}
		return Ok(this);
	}
}

use crate::utils::punctuated_args;
use syn::Expr;

#[derive(Default)]
pub struct ActionAttributes {
	pub global_observers: Vec<Expr>,
	pub observers: Vec<Expr>,
	pub systems: Vec<Expr>,
	pub category: Option<Expr>,
	pub storage: Option<Expr>,
}


impl ActionAttributes {
	pub fn parse(attrs: &[syn::Attribute]) -> syn::Result<Self> {
		let mut this = Self::default();

		for attr in attrs {
			let attr_str =
				attr.path().get_ident().map(|ident| ident.to_string());
			match attr_str.as_ref().map(|a| a.as_str()) {
				Some("global_observers") => {
					this.global_observers
						.extend(punctuated_args(attr.parse_args()?)?);
				}
				Some("observers") => {
					this.observers.extend(punctuated_args(attr.parse_args()?)?);
				}
				Some("systems") => {
					this.systems.extend(punctuated_args(attr.parse_args()?)?);
				}
				Some("category") => {
					this.category = Some(attr.parse_args()?);
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

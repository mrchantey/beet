use crate::utils::punctuated_args;
use syn::Expr;

pub struct ActionAttributes {
	pub global_observers: Vec<Expr>,
	pub observers: Vec<Expr>,
	pub systems: Vec<Expr>,
	pub category: Option<Expr>,
}


impl ActionAttributes {
	pub fn parse(attrs: &[syn::Attribute]) -> syn::Result<Self> {
		let mut global_observers = Vec::new();
		let mut observers = Vec::new();
		let mut systems = Vec::new();
		let mut category = None;

		for attr in attrs {
			let attr_str =
				attr.path().get_ident().map(|ident| ident.to_string());
			match attr_str.as_ref().map(|a| a.as_str()) {
				Some("global_observers") => {
					global_observers
						.extend(punctuated_args(attr.parse_args()?)?);
				}
				Some("observers") => {
					observers.extend(punctuated_args(attr.parse_args()?)?);
				}
				Some("systems") => {
					systems.extend(punctuated_args(attr.parse_args()?)?);
				}
				Some("category") => {
					category = Some(attr.parse_args()?);
				}
				_ => {}
			}
		}
		return Ok(Self {
			global_observers,
			observers,
			systems,
			category,
		});
	}
}

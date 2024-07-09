use crate::utils::punctuated_args;
use syn::Expr;

pub struct ActionAttributes {
	pub observers_generic: Vec<Expr>,
	pub observers_non_generic: Vec<Expr>,
	pub systems_generic: Vec<Expr>,
	pub systems_non_generic: Vec<Expr>,
	pub category: Option<Expr>,
	pub storage: Option<Expr>,
}


impl ActionAttributes {
	pub fn parse(attrs: &[syn::Attribute]) -> syn::Result<Self> {
		let mut observers_generic = Vec::new();
		let mut observers_non_generic = Vec::new();
		let mut systems_generic = Vec::new();
		let mut systems_non_generic = Vec::new();
		let mut category = None;
		let mut storage = None;

		for attr in attrs {
			let attr_str =
				attr.path().get_ident().map(|ident| ident.to_string());
			match attr_str.as_ref().map(|a| a.as_str()) {
				Some("observers") => {
					observers_non_generic
						.extend(punctuated_args(attr.parse_args()?)?);
				}
				Some("generic_observers") => {
					observers_generic
						.extend(punctuated_args(attr.parse_args()?)?);
				}
				Some("systems") => {
					systems_non_generic
						.extend(punctuated_args(attr.parse_args()?)?);
				}
				Some("generic_systems") => {
					systems_generic
						.extend(punctuated_args(attr.parse_args()?)?);
				}
				Some("category") => {
					category = Some(attr.parse_args()?);
				}
				Some("storage") => {
					storage = Some(attr.parse_args()?);
				}
				_ => {}
			}
		}
		return Ok(Self {
			observers_generic,
			observers_non_generic,
			systems_generic,
			systems_non_generic,
			category,
			storage,
		});
	}
}

pub struct ActionAttributes {
	pub observers: Vec<syn::Ident>,
	pub observers_non_generic: Vec<syn::Ident>,
}


impl ActionAttributes {
	pub fn parse(attrs: &[syn::Attribute]) -> syn::Result<Self> {
		let mut observers = Vec::new();
		let mut observers_non_generic = Vec::new();

		for attr in attrs {
			let attr_str =
				attr.path().get_ident().map(|ident| ident.to_string());
			match attr_str.as_ref().map(|a| a.as_str()) {
				Some("observers") => {
					observers.push(attr.parse_args()?);
				}
				Some("observers_non_generic") => {
					observers_non_generic.push(attr.parse_args()?);
				}
				_ => {}
			}
		}
		return Ok(Self {
			observers,
			observers_non_generic,
		});
	}
}

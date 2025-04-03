use beet_rsx::as_beet::*;

#[derive(Default, Buildable, IntoRsxAttributes)]
pub struct BaseHtmlAttributes {
	pub id: Option<String>,
	pub class: Option<String>,
}
#[derive(Default, Buildable, IntoRsxAttributes)]
pub struct ButtonHtmlAttributes {
	#[field(flatten)]
	pub base_attrs: BaseHtmlAttributes,
}
#[derive(Default, Buildable, IntoRsxAttributes)]
pub struct AnchorHtmlAttributes {
	#[field(flatten)]
	pub base_attrs: BaseHtmlAttributes,
	/// the download thing
	pub href: Option<String>,
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let a = AnchorHtmlAttributes::default().class("foo").href("bar");
		expect(a.base_attrs.class).to_be(Some("foo".to_string()));
		expect(a.href).to_be(Some("bar".to_string()));
	}
}

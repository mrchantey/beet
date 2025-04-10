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
	pub disabled: Option<bool>,
}
#[derive(Default, Buildable, IntoRsxAttributes)]
pub struct AnchorHtmlAttributes {
	#[field(flatten)]
	pub base_attrs: BaseHtmlAttributes,
	/// the download thing
	pub href: Option<String>,
}


pub trait BaseHtmlAttributesExt: BaseHtmlAttributesBuildable {
	fn push_class(&mut self, class: impl AsRef<str>) {
		if let Some(existing) = self.get_class_mut() {
			existing.push(' ');
			existing.push_str(class.as_ref());
		} else {
			self.set_class(class.as_ref());
		}
	}
}

impl<T: BaseHtmlAttributesBuildable> BaseHtmlAttributesExt for T {}


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


	#[test]
	fn third_order() {
		#[derive(Default, Buildable, IntoRsxAttributes)]
		struct Button {
			#[field(flatten)]
			#[field(flatten = BaseHtmlAttributes)]
			pub button_attrs: ButtonHtmlAttributes,
		}

		let _a = Button::default().class("foo").disabled(true);
	}
}

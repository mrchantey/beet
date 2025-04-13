use beet_rsx::as_beet::*;

#[derive(Default, Buildable, IntoBlockAttribute)]
pub struct BaseHtmlAttributes {
	pub id: Option<String>,
	pub class: Option<String>,
	pub onchange: Option<Box<dyn EventHandler<Event>>>,
	pub onclick: Option<Box<dyn EventHandler<MouseEvent>>>,
}
#[derive(Default, Buildable, IntoBlockAttribute)]
pub struct ButtonHtmlAttributes {
	#[field(flatten)]
	pub base_attrs: BaseHtmlAttributes,
	pub disabled: Option<bool>,
}


#[derive(Default, Buildable, IntoBlockAttribute)]
pub struct AnchorHtmlAttributes {
	// #[field(flatten=BaseHtmlAttributes)]
	#[field(flatten)]
	pub base_attrs: BaseHtmlAttributes,
	/// the download thing
	pub href: Option<String>,
}
#[derive(Default, Buildable, IntoBlockAttribute)]
pub struct InputHtmlAttributes {
	#[field(flatten)]
	pub base_attrs: BaseHtmlAttributes,
	pub r#type: Option<String>,
	pub disabled: Option<bool>,
	pub required: Option<bool>,
	// #[field(into)]
	// pub value: Option<MaybeSignal<String>>,
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
		#[derive(Default, Buildable, IntoBlockAttribute)]
		struct Button {
			#[field(flatten)]
			#[field(flatten = BaseHtmlAttributes)]
			pub button_attrs: ButtonHtmlAttributes,
		}

		let _a = Button::default().class("foo").disabled(true);
	}

	#[test]
	fn events_omitted() {
		#[derive(Node)]
		struct Button {
			#[field(flatten = BaseHtmlAttributes)]
			_button_attrs: ButtonHtmlAttributes,
		}
		fn button(_props: Button) -> RsxNode { Default::default() }
		// onclick was ommitted from the into_rsx_attributes
		let _foo = rsx! { <Button onclick=|_| {} /> };
	}
}

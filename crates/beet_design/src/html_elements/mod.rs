use beet_template::as_beet::*;

#[derive(Default, Buildable, TemplateBundle)]
pub struct BaseHtmlAttributes {
	pub id: Option<String>,
	pub class: Option<String>,
	pub onchange: Option<EventHandler<OnChange>>,
	pub oninput: Option<EventHandler<OnInput>>,
	pub onclick: Option<EventHandler<OnClick>>,
}
#[derive(Default, Buildable, TemplateBundle)]
pub struct ButtonHtmlAttributes {
	#[field(flatten)]
	pub base_attrs: BaseHtmlAttributes,
	pub disabled: Option<bool>,
}


#[derive(Default, Buildable, TemplateBundle)]
pub struct AnchorHtmlAttributes {
	// #[field(flatten=BaseHtmlAttributes)]
	#[field(flatten)]
	pub base_attrs: BaseHtmlAttributes,
	/// the download thing
	pub href: Option<String>,
}
#[derive(Default, Buildable, TemplateBundle)]
pub struct InputHtmlAttributes {
	#[field(flatten)]
	pub base_attrs: BaseHtmlAttributes,
	pub r#type: Option<String>,
	pub disabled: Option<bool>,
	pub required: Option<bool>,
	pub value: Option<MaybeSignal<String>>,
}

#[derive(Default, Buildable, TemplateBundle)]
pub struct TextAreaHtmlAttributes {
	#[field(flatten=BaseHtmlAttributes)]
	pub input_attrs: InputHtmlAttributes,
	pub rows: Option<u32>,
	pub cols: Option<u32>,
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
		#[derive(Default, Buildable, TemplateBundle)]
		struct Button {
			#[field(flatten)]
			#[field(flatten = BaseHtmlAttributes)]
			pub button_attrs: ButtonHtmlAttributes,
		}

		let _a = Button::default().class("foo").disabled(true);
	}

	#[test]
	// #[allow(unused)]
	fn events_omitted() {
		#[template]
		fn Button(
			#[field(flatten = BaseHtmlAttributes)]
			_button_attrs: ButtonHtmlAttributes,
		) -> impl Bundle {
			()
		}
		// onclick was ommitted from the into_rsx_attributes
		let _foo = rsx! { <Button onclick=|_| {} /> };
	}
}

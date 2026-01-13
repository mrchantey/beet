// TODO use webidl or something to autogenerate these
// possibly use templates instead
use crate::prelude::*;

#[derive(Default, Buildable, AttributeBlock)]
pub struct BaseHtmlAttributes {
	pub id: Option<String>,
	pub class: Option<String>,
	pub r#type: Option<String>,
	pub name: Option<String>,
	pub tabindex: Option<String>,
	pub autofocus: Option<bool>,
	pub onchange: Option<EventHandler<OnChange>>,
	pub oninput: Option<EventHandler<OnInput>>,
	pub onclick: Option<EventHandler<OnClick>>,
	pub onkeydown: Option<EventHandler<OnKeyDown>>,
}
#[derive(Default, Buildable, AttributeBlock)]
pub struct ButtonHtmlAttributes {
	#[field(flatten)]
	pub base_attrs: BaseHtmlAttributes,
	pub disabled: Option<DerivedGetter<bool>>,
}


#[derive(Default, Buildable, AttributeBlock)]
pub struct AnchorHtmlAttributes {
	// #[field(flatten=BaseHtmlAttributes)]
	#[field(flatten)]
	pub base_attrs: BaseHtmlAttributes,
	/// the download thing
	pub href: Option<String>,
}
#[derive(Default, Buildable, AttributeBlock)]
pub struct InputHtmlAttributes {
	#[field(flatten)]
	pub base_attrs: BaseHtmlAttributes,
	pub disabled: Option<DerivedGetter<bool>>,
	pub required: Option<bool>,
	pub value: Option<DerivedGetter<String>>,
}

#[derive(Default, Buildable, AttributeBlock)]
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
	use beet_core::prelude::*;



	#[test]
	fn works() {
		let a = AnchorHtmlAttributes::default().class("foo").href("bar");
		a.base_attrs.class.xpect_eq(Some("foo".to_string()));
		a.href.xpect_eq(Some("bar".to_string()));
	}


	#[test]
	fn third_order() {
		#[derive(Default, Buildable, AttributeBlock)]
		struct Button {
			#[field(flatten)]
			#[field(flatten = BaseHtmlAttributes)]
			pub button_attrs: ButtonHtmlAttributes,
		}

		let _a = Button::default().class("foo").disabled(true);
	}

	#[template]
	fn Button(
		#[field(flatten = BaseHtmlAttributes)] attrs: ButtonHtmlAttributes,
	) -> impl Bundle {
		rsx! { <button {attrs} /> }
	}
	#[test]
	fn renders() {
		rsx! { <Button disabled=true id="foo" onclick=|_| {} /> }
			.xmap(HtmlDocument::parse_bundle)
			.xpect_contains(r#"<button disabled="true" id="foo" onclick="_beet_event_handler(0, event)" data-beet-dom-idx="0"/>"#);
	}
}

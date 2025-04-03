use beet_rsx::as_beet::*;

#[derive(Default, Buildable, IntoRsxAttributes)]
pub struct BaseHtmlAttributes {
	pub class: Option<String>,
}


pub trait HasHtmlAttributes: AsMut<BaseHtmlAttributes> + Sized {
	fn class(mut self, class: String) -> Self {
		let this = self.as_mut();
		this.class = Some(class);
		self
	}
}
impl<T> HasHtmlAttributes for T where T: AsMut<BaseHtmlAttributes> {}

// pub trait HtmlAttributesBuilder: Sized {
// 	fn class(self, class: &str) -> Self;
// }


#[cfg(test)]
mod test {
	use crate::prelude::*;
	// use beet_rsx::as_beet::*;
	// use sweet::prelude::*;

	#[derive(Node, Default)]
	struct MyComponent {
		my_field: bool,
		// #[field(flatten)]
		// html_attributes: BaseHtmlAttributes,
	}

	impl AsMut<bool> for MyComponent {
		fn as_mut(&mut self) -> &mut bool { &mut self.my_field }
	}

	fn my_component(_: MyComponent) -> RsxNode { RsxNode::default() }

	#[test]
	fn works() {

		// let a = rsx!{
		// 	<MyComponent class="foobar" />
		// };
		// let a = MyComponent::default().class("foobar".to_string());
		// expect(a.html_attributes.class).to_be(Some("foobar".to_string()))
	}
}

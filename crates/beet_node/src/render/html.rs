use crate::prelude::*;
use beet_core::prelude::*;
use std::borrow::Cow;

pub struct HtmlRenderer {
	/// Elements without a closing tag, and whose children
	/// will be popped out to trailing siblings
	pub void_elements: Vec<Cow<'static, str>>,
}

impl Default for HtmlRenderer {
	fn default() -> Self {
		Self {
			void_elements: vec![
				"area".into(),
				"base".into(),
				"br".into(),
				"col".into(),
				"embed".into(),
				"hr".into(),
				"img".into(),
				"input".into(),
				"link".into(),
				"meta".into(),
				"param".into(),
				"source".into(),
				"track".into(),
				"wbr".into(),
			],
		}
	}
}

impl NodeVisitor for HtmlRenderer {
	fn visit_element(
		&mut self,
		_cx: &VisitContext,
		_element: &Element,
		_attributes: Vec<(Entity, &Attribute, &Value)>,
	) {
		todo!()
	}

	fn visit_value(&mut self, _cx: &VisitContext, _value: &Value) { todo!() }
	fn leave_element(&mut self, _cx: &VisitContext, _element: &Element) {
		todo!()
	}
}

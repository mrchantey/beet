use beet_rsx::prelude::*;

pub struct Style {
	pub css: String,
	pub attrs: Vec<RsxAttribute>,
}

impl IntoRsxRoot for Style {
	fn into_root(self) -> RsxRoot {
		RsxElement {
			tag: "style".to_string(),
			attributes: self.attrs,
			self_closing: false,
			children: Box::new(RsxNode::Text { value: self.css }),
		}
		.into_root()
	}
}

impl Style {
	pub fn new(css: impl ToString) -> Self {
		Self {
			css: css.to_string(),
			attrs: vec![],
		}
	}
	pub fn with_global_scope(mut self) -> Self {
		self.add_attr("scope:global");
		self
	}
	pub fn add_attr<M>(&mut self, attr: impl IntoRsxAttribute<M>) {
		self.attrs.push(attr.into_rsx_attribute());
	}
}

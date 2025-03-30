use beet_rsx::prelude::*;

pub struct Style {
	pub css: String,
	pub attrs: Vec<RsxAttribute>,
}

impl IntoRsxNode for Style {
	fn into_node(self) -> RsxNode {
		RsxElement {
			tag: "style".to_string(),
			attributes: self.attrs,
			self_closing: false,
			children: Box::new(self.css.into_node()),
			location: None,
		}
		.into_node()
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

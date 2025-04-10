use beet_rsx::prelude::*;

pub struct Style {
	pub css: String,
	pub directives: Vec<TemplateDirective>,
}

impl IntoRsxNode for Style {
	fn into_node(self) -> RsxNode {
		RsxElement {
			tag: "style".to_string(),
			attributes: Default::default(),
			self_closing: false,
			children: Box::new(self.css.into_node()),
			meta: RsxNodeMeta {
				template_directives: self.directives,
				location: Default::default(),
			},
		}
		.into_node()
	}
}

impl Style {
	pub fn new(css: impl ToString) -> Self {
		Self {
			css: css.to_string(),
			directives: Default::default(),
		}
	}
	pub fn with_directive(mut self, directive: TemplateDirective) -> Self {
		self.directives.push(directive);
		self
	}
}

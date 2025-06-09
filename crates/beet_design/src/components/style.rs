use beet_common::prelude::*;
use beet_template::prelude::*;

// what does this do? i think it can be removed since we have
// is:inline now?
pub struct Style {
	pub css: String,
	pub directives: Vec<TemplateDirectiveEnum>,
}

impl IntoWebNode for Style {
	fn into_node(self) -> WebNode {
		RsxElement {
			tag: "style".to_string(),
			attributes: Default::default(),
			self_closing: false,
			children: Box::new(self.css.into_node()),
			meta: NodeMeta::new(FileSpan::default(), self.directives),
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
	pub fn with_directive(mut self, directive: TemplateDirectiveEnum) -> Self {
		self.directives.push(directive);
		self
	}
}

use crate::prelude::*;
use beet_common::prelude::*;


pub struct RemoveLangTemplates {
	pub tags: Vec<String>,
}

impl Default for RemoveLangTemplates {
	fn default() -> Self {
		Self {
			tags: Self::default_tags(),
		}
	}
}


impl RemoveLangTemplates {
	pub fn default_tags() -> Vec<String> {
		vec!["style".to_string(), "script".to_string()]
	}
}


impl Pipeline<WebNode, WebNode> for RemoveLangTemplates {
	fn apply(self, mut node: WebNode) -> WebNode {
		VisitWebNodeMut::walk(&mut node, |node| {
			// logic must be consistent with ExtractLangTemplates
			if let WebNode::Element(el) = node
				&& !el.is_inline()
				&& self.tags.contains(&el.tag)
			{
				sweet::log!("remove lang template");
				*node = Default::default();
			}
		});
		node
	}
}

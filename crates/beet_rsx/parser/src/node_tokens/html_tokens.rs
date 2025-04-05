use std::ops::ControlFlow;

use crate::prelude::*;


/// Html custom node tokens
#[derive(Debug, Clone)]
pub enum HtmlNodeTokens {
	Doctype {
		directives: Vec<TemplateDirectiveTokens>,
	},
	Comment {
		text: String,
		directives: Vec<TemplateDirectiveTokens>,
	},
}

impl AsRef<Vec<TemplateDirectiveTokens>> for HtmlNodeTokens {
	fn as_ref(&self) -> &Vec<TemplateDirectiveTokens> {
		match self {
			HtmlNodeTokens::Doctype { directives, .. }
			| HtmlNodeTokens::Comment { directives, .. } => directives,
		}
	}
}

impl Into<NodeTokens<Self>> for HtmlNodeTokens {
	fn into(self) -> NodeTokens<Self> {
		match self {
			HtmlNodeTokens::Doctype { directives } => {
				NodeTokens::Custom(HtmlNodeTokens::Doctype { directives })
			}
			HtmlNodeTokens::Comment { text, directives } => {
				NodeTokens::Custom(HtmlNodeTokens::Comment { text, directives })
			}
		}
	}
}


impl CustomNodeTokens for HtmlNodeTokens {
	type RstmlParser = HtmlRstmlParser;
	type CustomRstmlNode = rstml::Infallible;
}


pub struct HtmlRstmlParser;

impl RstmlParser for HtmlRstmlParser {
	type NodeTokens = HtmlNodeTokens;
	fn map_node(
		&mut self,
		node: RstmlNode<Self::CustomRstmlNode>,
	) -> ControlFlow<
		NodeTokens<Self::NodeTokens>,
		RstmlNode<Self::CustomRstmlNode>,
	> {
		match node {
			RstmlNode::Comment(comment) => ControlFlow::Break(
				HtmlNodeTokens::Comment {
					directives: Default::default(),
					text: comment.value.value(),
				}
				.into(),
			),
			RstmlNode::Doctype(_) => ControlFlow::Break(
				HtmlNodeTokens::Doctype {
					directives: Default::default(),
				}
				.into(),
			),
			_ => ControlFlow::Continue(node),
		}
	}
}

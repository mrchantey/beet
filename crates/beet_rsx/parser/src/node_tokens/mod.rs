mod rstml_parser;
pub use rstml_parser::*;
mod html_tokens;
pub use html_tokens::*;
mod node_tokens;
pub use node_tokens::*;
///
pub trait CustomNodeTokens {
	// rstml
	type RstmlParser: RstmlParser<NodeTokens = Self, CustomRstmlNode = Self::CustomRstmlNode>;
	type CustomRstmlNode: rstml::node::CustomNode + std::fmt::Debug;
}


impl CustomNodeTokens for () {
	type RstmlParser = ();
	type CustomRstmlNode = rstml::Infallible;
}

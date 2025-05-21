#![allow(unused)]

use crate::prelude::*;
use beet_common::prelude::*;
use proc_macro2::TokenStream;
use proc_macro2_diagnostics::Diagnostic;
use proc_macro2_diagnostics::Level;
use rstml::node::CustomNode;
use rstml::node::Node;
use rstml::node::NodeAttribute;
use rstml::node::NodeBlock;
use rstml::node::NodeElement;
use rstml::node::NodeName;
use std::collections::HashSet;
use sweet::prelude::Pipeline;
use sweet::prelude::WorkspacePathBuf;
use syn::Expr;
use syn::LitStr;
use syn::spanned::Spanned;



/// Convert rstml nodes to a Vec<WebNode> token stream
/// ## Pipeline
/// [`Pipeline<Vec<Node<C>>, (WebTokens, Vec<TokenStream>)>`]
#[derive(Debug)]
pub struct RstmlToWebTokens<C = rstml::Infallible> {
	// Additional error and warning messages.
	pub errors: Vec<TokenStream>,
	// Collect elements to provide semantic highlight based on element tag.
	// No differences between open tag and closed tag.
	// Also multiple tags with same name can be present,
	// because we need to mark each of them.
	pub collected_elements: Vec<NodeName>,
	// rstml requires std hashset :(
	self_closing_elements: HashSet<&'static str>,
	phantom: std::marker::PhantomData<C>,
	/// The span of the entry node, this will be taken
	/// by the first node visited.
	file: WorkspacePathBuf,
	// Collect elements to provide semantic highlight based on element tag.
	// No differences between open tag and closed tag.
	// Also multiple tags with same name can be present,
	// because we need to mark each of them.
	pub rusty_tracker: RustyTrackerBuilder,
}

impl Default for RstmlToWebTokens<rstml::Infallible> {
	fn default() -> Self {
		Self {
			file: WorkspacePathBuf::default(),
			errors: Vec::new(),
			collected_elements: Vec::new(),
			self_closing_elements: self_closing_elements(),
			phantom: std::marker::PhantomData,
			rusty_tracker: RustyTrackerBuilder::default(),
		}
	}
}

impl RstmlToWebTokens<rstml::Infallible> {
	pub fn new(file: WorkspacePathBuf) -> Self {
		Self {
			file,
			..Default::default()
		}
	}
}

/// Parse rstml nodes to a [`NodeTokens`] and any compile errors
impl<C: CustomNode> Pipeline<Vec<Node<C>>, (WebTokens, Vec<TokenStream>)>
	for RstmlToWebTokens<C>
{
	fn apply(mut self, _nodes: Vec<Node<C>>) -> (WebTokens, Vec<TokenStream>) {
		(Default::default(), Vec::new())
	}
}

use beet_common::prelude::*;
use rapidhash::RapidHasher;
use serde::Deserialize;
use serde::Serialize;
use std::hash::Hash;
use std::hash::Hasher;
use sweet::prelude::WorkspacePathBuf;

// use std::sync::atomic::AtomicUsize;
// use std::sync::atomic::Ordering;

// static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

// fn next_id() -> usize { ID_COUNTER.fetch_add(1, Ordering::Relaxed) }



#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LangTemplate {
	/// the scope of the style
	pub directives: Vec<TemplateDirective>,
	/// the child text of the element, may be empty
	/// for src templates
	pub content: LangContent,
}

impl LangTemplate {
	/// Hash the content of the template
	pub fn hash_self(&self) -> u64 {
		let mut hasher = RapidHasher::default_const();
		self.hash(&mut hasher);
		hasher.finish()
	}
}

/// The content of a style template, either inline or a file path
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LangContent {
	/// Inner text of an elment: `<script>alert("hello")</script>`
	Inline(String),
	/// A path to a file: `<script src="./foo.js" />`
	File(WorkspacePathBuf),
}

impl LangContent {}


impl LangTemplate {
	pub fn new(
		directives: Vec<TemplateDirective>,
		content: LangContent,
	) -> Self {
		Self {
			directives,
			content,
		}
	}
}

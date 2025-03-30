use crate::prelude::*;
use std::borrow::Borrow;
use std::borrow::BorrowMut;

/// This is an RsxNode and a location, which is required for hydration.
///
/// It is allowed for the [`RsxRoot`] to be default(), which means that
/// the macro location is a placeholder, this means that the the node
/// will not be eligible for nested templating etc. which is the case
/// anyway for Strings and ().
///
/// The struct returned from an rsx! macro.
#[derive(Debug, Clone, Default)]
pub struct RsxRoot {
	/// the root node
	pub node: RsxNode,
	/// Unique location with file, line, col.
	/// The primary purpose is for applying templates,
	/// so is usually only present when the macro is used
	pub location: Option<RsxMacroLocation>,
}

impl RsxRoot {
	pub fn location_str(&self) -> String {
		match self.location {
			Some(ref loc) => loc.to_string(),
			None => "<unknown>".to_string(),
		}
	}
}

impl std::ops::Deref for RsxRoot {
	type Target = RsxNode;
	fn deref(&self) -> &Self::Target { &self.node }
}
impl std::ops::DerefMut for RsxRoot {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.node }
}

impl AsRef<RsxNode> for RsxRoot {
	fn as_ref(&self) -> &RsxNode { &self.node }
}
impl AsMut<RsxNode> for RsxRoot {
	fn as_mut(&mut self) -> &mut RsxNode { &mut self.node }
}
// This also means RsxRoot: impl IntoRsxRoot
impl Into<RsxNode> for RsxRoot {
	fn into(self) -> RsxNode { self.node }
}

impl Borrow<RsxNode> for RsxRoot {
	fn borrow(&self) -> &RsxNode { &self.node }
}
impl BorrowMut<RsxNode> for RsxRoot {
	fn borrow_mut(&mut self) -> &mut RsxNode { &mut self.node }
}

pub trait IntoRsxRoot<M = ()> {
	fn into_root(self) -> RsxRoot;
}

impl IntoRsxRoot<()> for () {
	fn into_root(self) -> RsxRoot { RsxRoot::default() }
}

pub struct NodeToRoot;
impl<T: Into<RsxNode>> IntoRsxRoot<(T, NodeToRoot)> for T {
	fn into_root(self) -> RsxRoot {
		RsxRoot {
			location: None,
			node: self.into(),
		}
	}
}
pub struct FuncIntoRsx;
impl<T: FnOnce() -> U, U: IntoRsxRoot<M2>, M2> IntoRsxRoot<(M2, FuncIntoRsx)>
	for T
{
	fn into_root(self) -> RsxRoot { self().into_root() }
}

pub struct VecIntoRsx;
impl<T: IntoRsxRoot<M2>, M2> IntoRsxRoot<(M2, VecIntoRsx)> for Vec<T> {
	fn into_root(self) -> RsxRoot {
		let node = RsxNode::Fragment {
			nodes: self.into_iter().map(|item| item.into_root().node).collect(),
		};
		RsxRoot {
			node,
			..Default::default()
		}
	}
}

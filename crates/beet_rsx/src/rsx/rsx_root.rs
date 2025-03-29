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
#[derive(Debug, Default)]
pub struct RsxRoot {
	/// the root node
	pub node: RsxNode,
	/// unique location with file, line, col
	pub location: RsxMacroLocation,
}

impl RsxRoot {}

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

impl Into<RsxNode> for RsxRoot {
	fn into(self) -> RsxNode { self.node }
}

impl Borrow<RsxNode> for RsxRoot {
	fn borrow(&self) -> &RsxNode { &self.node }
}
impl BorrowMut<RsxNode> for RsxRoot {
	fn borrow_mut(&mut self) -> &mut RsxNode { &mut self.node }
}

pub trait IntoRsxRoot<M> {
	fn into_root(self) -> RsxRoot;
}
impl IntoRsxRoot<RsxRoot> for RsxRoot {
	fn into_root(self) -> RsxRoot { self }
}

impl IntoRsxRoot<()> for () {
	fn into_root(self) -> RsxRoot { RsxRoot::default() }
}

/// Strings are allowed to have an RsxMacroLocation::default(),
/// as they will never be used for complex hydration etc
// TODO its a code smell that we have to do this
pub struct ToStringIntoRsx;
impl<T: ToString> IntoRsxRoot<(T, ToStringIntoRsx)> for T {
	fn into_root(self) -> RsxRoot {
		RsxRoot {
			location: RsxMacroLocation::default(),
			node: RsxNode::Text {
				idx: RsxIdx::default(),
				value: self.to_string(),
			},
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
			idx: RsxIdx::default(),
			nodes: self.into_iter().map(|item| item.into_root().node).collect(),
		};
		RsxRoot {
			node,
			..Default::default()
		}
	}
}

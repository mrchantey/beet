/// Some extra metadata used to indicate the purpose of an action, ie which parts of the world it will effect.
/// This is **not** used at runtime, only for UI and debugging purposes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GraphRole {
	/// This action will effect only this node
	Node,
	/// This action will effect children of this node
	Child,
	/// This action will effect the agent bound to this node
	Agent,
	/// This action will effect some other aspect of the world
	Other,
}

impl GraphRole {}

#[derive(Debug, Clone, PartialEq)]
pub enum GraphRole {
	Parent,
	Node,
	Agent,
	Child,
	Other,
}


pub trait ActionMeta {
	fn graph_role() -> GraphRole;
}

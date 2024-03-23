#[derive(Debug, Clone, PartialEq)]
pub enum GraphRole {
	Parent,
	Node,
	Agent,
	Child,
	Other,
	Multi(Vec<GraphRole>),
}


pub trait ActionMeta {
	fn graph_role() -> GraphRole;
}

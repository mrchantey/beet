use crate::prelude::*;

#[allow(unused_variables)]
pub trait TreeVisitor {
	type Node: 'static + Tree;
	type Err;
	fn walk_nodes_dfs<'a>(
		&mut self,
		nodes: impl IntoIterator<Item = &'a Self::Node>,
	) -> Result<(), Self::Err> {
		for node in nodes.into_iter() {
			self.visit_node(node)?;
			self.walk_nodes_dfs(node.children())?;
			self.leave_node(node)?;
		}
		Ok(())
	}
	fn visit_node(&mut self, node: &Self::Node) -> Result<(), Self::Err> {
		Ok(())
	}
	fn leave_node(&mut self, node: &Self::Node) -> Result<(), Self::Err> {
		Ok(())
	}
}

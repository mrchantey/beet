use beet_ecs::prelude::*;
use petgraph::graph::DiGraph;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let tree = Tree::new(7).with_leaf(8).with_leaf(89);
	let graph = DiGraph::from_tree(tree.clone());
	expect(graph.into_tree()).to_be(tree)?;

	Ok(())
}

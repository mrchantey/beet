use beet_ecs::prelude::*;
use petgraph::graph::Graph;
use petgraph::stable_graph::NodeIndex;
use sweet::*;

#[sweet_test]
pub fn is_identical() -> Result<()> {
	let mut graph1 = Graph::<i32, ()>::new();
	graph1.add_node(0);
	let mut graph2 = Graph::<i32, ()>::new();
	graph2.add_node(0);

	expect(graph1.is_identical(&graph2)).to_be_true()?;
	graph1.add_node(7);
	expect(graph1.is_identical(&graph2)).to_be_false()?;
	graph2.add_node(7);
	expect(graph1.is_identical(&graph2)).to_be_true()?;
	graph1.add_edge(NodeIndex::new(0), NodeIndex::new(1), ());
	expect(graph1.is_identical(&graph2)).to_be_false()?;

	Ok(())
}
#[sweet_test]
pub fn remove_node_recursive() -> Result<()> {
	let mut graph = Graph::<i32, ()>::new();
	for i in 0..10 {
		graph.add_node(i);
	}

	for i in 0..9 {
		graph.add_edge(NodeIndex::new(i), NodeIndex::new(i + 1), ());
	}

	graph.remove_node_recursive(NodeIndex::new(5));
	expect(graph.node_count()).to_be(5)?;
	Ok(())
}

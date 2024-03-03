use crate::prelude::*;
use bevy_utils::HashMap;
use bevy_utils::HashSet;
use extend::ext;
use petgraph::graph::DiGraph;
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::IndexType;
use petgraph::visit::Dfs;
use petgraph::visit::Walker;
use petgraph::Direction;
use petgraph::EdgeType;
use petgraph::Graph;
use std::iter::Rev;
#[ext(name=GraphExt)]
pub impl<N, E, Ty, Ix> Graph<N, E, Ty, Ix>
where
	Ty: EdgeType,
	Ix: IndexType,
{
	/// Provides neighbors in the order they were added.
	/// This is the reverse of [`Graph::neighbors_directed`]
	fn neighbors_directed_in_order(
		&self,
		parent: NodeIndex<Ix>,
		direction: Direction,
	) -> Rev<std::vec::IntoIter<NodeIndex<Ix>>> {
		self.neighbors_directed(parent, direction)
			.collect::<Vec<_>>()
			.into_iter()
			.rev()
	}
}

#[ext]
pub impl<N, E, Ty, Ix> Graph<N, E, Ty, Ix>
where
	N: PartialEq,
	E: PartialEq,
	Ty: petgraph::EdgeType,
	Ix: petgraph::graph::IndexType + PartialEq,
{
	/// This is a strict equality, nodes and edges must be identical in order
	fn is_identical(&self, b: &Self) -> bool {
		let a_ns = self.raw_nodes().iter().map(|n| &n.weight);
		let b_ns = b.raw_nodes().iter().map(|n| &n.weight);
		let a_es = self
			.raw_edges()
			.iter()
			.map(|e| (e.source(), e.target(), &e.weight));
		let b_es = b
			.raw_edges()
			.iter()
			.map(|e| (e.source(), e.target(), &e.weight));
		a_ns.eq(b_ns) && a_es.eq(b_es)
	}

	fn remove_node_recursive(&mut self, index: NodeIndex<Ix>) {
		let to_remove =
			Dfs::new(&*self, index).iter(&*self).collect::<HashSet<_>>();
		self.retain_nodes(|_, node| !to_remove.contains(&node));
	}
}


#[ext]
pub impl<N, E> DiGraph<N, E> {
	fn root(&self) -> Option<&N> { self.node_weight(NodeIndex::new(0)) }
	fn node(&self, index: usize) -> Option<&N> {
		self.node_weight(NodeIndex::new(index))
	}
}

#[ext(name=DiGraphExtUnitEdge)]
pub impl<N> DiGraph<N, ()> {
	fn from_tree(root: Tree<N>) -> Self {
		let mut this = Self::new();
		this.from_tree_recursive(root.into());
		this
	}

	fn from_tree_recursive(&mut self, tree: Tree<N>) -> NodeIndex {
		let Tree::<N> { value, children } = tree;
		let node = self.add_node(value);

		for child in children.into_iter() {
			let index = self.from_tree_recursive(child);
			self.add_edge(node, index, ());
		}

		node
	}

	/// Take nodes from a graph, preserving node indices.
	fn take_nodes(mut self) -> HashMap<NodeIndex, N> {
		let mut nodes = HashMap::default();
		// reverse to safely remove
		for index in self.node_indices().rev() {
			nodes.insert(index, self.remove_node(index).unwrap());
		}
		nodes
	}


	/// Discards edges to previously visited nodes (dfs).
	fn into_tree(self) -> Tree<N> {
		if self.node_count() == 0 {
			panic!("Cannot convert empty graph to tree");
		}

		let index_tree = self
			.index_tree_recursive(NodeIndex::new(0), &mut HashSet::default());
		let mut node_map = self.take_nodes();
		Self::index_tree_to_tree(index_tree, &mut node_map)
	}

	fn index_tree_to_tree(
		index_tree: Tree<NodeIndex>,
		node_map: &mut HashMap<NodeIndex, N>,
	) -> Tree<N> {
		let Tree::<NodeIndex> { value, children } = index_tree;
		let children = children
			.into_iter()
			.map(|child| Self::index_tree_to_tree(child, node_map))
			.collect::<Vec<_>>();

		Tree {
			value: node_map.remove(&value).unwrap(),
			children,
		}
	}


	/// Note: This will empty the graph.
	fn index_tree_recursive(
		&self,
		parent: NodeIndex,
		visited: &mut HashSet<NodeIndex>,
	) -> Tree<NodeIndex> {
		visited.insert(parent);

		Tree {
			value: parent,
			children: self
				.neighbors_directed_in_order(parent, Direction::Outgoing)
				.filter(|index| !visited.contains(index))
				.collect::<Vec<_>>()
				.into_iter()
				.map(|index| self.index_tree_recursive(index, visited))
				.collect(),
		}
	}
}

// deprecated in favour of into_tree

// #[ext(name=DiGraphExtPartialEq)]
// pub impl<N, E> DiGraph<N, E>
// where
// 	N: PartialEq,
// {
// 	/// Untested on graphs that have had edges removed and re-added.
// 	fn equals_tree(&self, tree: &Tree<N>) -> bool {
// 		self.equals_tree_recursive(tree, NodeIndex::default())
// 	}
// 	fn equals_tree_recursive(&self, tree: &Tree<N>, start: NodeIndex) -> bool {
// 		if self.node_weight(start) != Some(&tree.value) {
// 			false
// 		} else {
// 			self.neighbors(start).enumerate().all(|(i, child)| {
// 				self.equals_tree_recursive(&tree.children[i], child)
// 			})
// 		}
// 	}
// }

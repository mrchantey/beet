use crate::prelude::*;

impl RsxNode {
	/// Depth first traversal of the tree
	pub fn visit(&self, mut func: impl FnMut(&RsxNode)) {
		fn inner(node: &RsxNode, func: &mut impl FnMut(&RsxNode)) {
			func(node);
			for child in node.children() {
				inner(child, func);
			}
		}
		inner(self, &mut func);
	}
	/// Depth first mutable traversal of the tree
	pub fn visit_mut(&mut self, mut func: impl FnMut(&mut RsxNode)) {
		fn inner(node: &mut RsxNode, func: &mut impl FnMut(&mut RsxNode)) {
			func(node);
			for child in node.children_mut() {
				inner(child, func);
			}
		}
		inner(self, &mut func);
	}

	/// Depth first traversal of the tree, will not visit child components,
	/// this is useful for patterns like [ScopedStyle]
	pub fn visit_ignore_components(&self, mut func: impl FnMut(&RsxNode)) {
		fn inner(node: &RsxNode, func: &mut impl FnMut(&RsxNode)) {
			if let RsxNode::Component { .. } = node {
				return;
			}
			func(node);
			for child in node.children() {
				inner(child, func);
			}
		}
		inner(self, &mut func);
	}
	/// Depth first mutable traversal of the tree, will not visit child components,
	/// this is useful for patterns like [ScopedStyle]
	pub fn visit_ignore_components_mut(
		&mut self,
		mut func: impl FnMut(&mut RsxNode),
	) {
		fn inner(node: &mut RsxNode, func: &mut impl FnMut(&mut RsxNode)) {
			if let RsxNode::Component { .. } = node {
				return;
			}
			func(node);
			for child in node.children_mut() {
				inner(child, func);
			}
		}
		inner(self, &mut func);
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	struct Child;
	impl Component for Child {
		fn render(self) -> RsxRoot {
			rsx! {<div/><slot/>}
		}
	}

	#[test]
	fn visit_ignore_components() {
		let mut count = 0;
		rsx! {
			<div>
				<Child>
					<br/>
				</Child>
				<br/>
			</div>
		}
		.visit_ignore_components(|_| {
			count += 1;
		});
		expect(count).to_be(2);
	}
	#[test]
	fn visit_ignore_components_mut() {
		let mut count = 0;
		rsx! {
			<div>
				<Child>
					<br/>
				</Child>
				<br/>
			</div>
		}
		.visit_ignore_components_mut(|_| {
			count += 1;
		});
		expect(count).to_be(2);
	}
}

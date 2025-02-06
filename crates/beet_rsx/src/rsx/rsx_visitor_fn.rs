use crate::prelude::*;

#[derive(Debug, Default, Clone)]
pub struct VisitRsxOptions {
	ignore_element_children: bool,
	ignore_component_node: bool,
	ignore_component_slot_children: bool,
}

impl VisitRsxOptions {
	pub fn ignore_component_node() -> Self {
		Self {
			ignore_component_node: true,
			..Default::default()
		}
	}
}

macro_rules! impl_visitor {
	($visitor_name:ident, $node_type:ty, $visit_method:ident) => {
		/// Convenience struct for when only one type needs to be visited
		pub struct $visitor_name<F> {
			func: F,
			options: VisitRsxOptions,
		}

		impl<F: FnMut(&$node_type)> $visitor_name<F> {
			pub fn walk(node: &RsxNode, func: F) {
				$visitor_name::new(func).walk_node(node);
			}
			pub fn walk_with_opts(
				node: &RsxNode,
				options: VisitRsxOptions,
				func: F,
			) {
				$visitor_name::new_with_options(options, func).walk_node(node);
			}


			pub fn new(func: F) -> Self {
				Self {
					func,
					options: Default::default(),
				}
			}

			pub fn new_with_options(options: VisitRsxOptions, func: F) -> Self {
				Self { func, options }
			}
		}

		impl<F: FnMut(&$node_type)> RsxVisitor for $visitor_name<F> {
			fn ignore_element_children(&self) -> bool {
				self.options.ignore_element_children
			}

			fn ignore_component_node(&self) -> bool {
				self.options.ignore_component_node
			}

			fn ignore_component_slot_children(&self) -> bool {
				self.options.ignore_component_slot_children
			}

			fn $visit_method(&mut self, value: &$node_type) {
				(self.func)(value);
			}
		}
	};
}

macro_rules! impl_visitor_mut {
	($visitor_name:ident, $node_type:ty, $visit_method:ident) => {
		/// Convenience struct for when only one type needs to be visited
		pub struct $visitor_name<F> {
			func: F,
			options: VisitRsxOptions,
		}

		impl<F: FnMut(&mut $node_type)> $visitor_name<F> {
			pub fn new(func: F) -> Self {
				Self {
					func,
					options: Default::default(),
				}
			}
			pub fn walk(node: &mut RsxNode, func: F) {
				$visitor_name::new(func).walk_node(node);
			}
			pub fn walk_with_opts(
				node: &mut RsxNode,
				options: VisitRsxOptions,
				func: F,
			) {
				$visitor_name::new_with_options(options, func).walk_node(node);
			}


			pub fn new_with_options(options: VisitRsxOptions, func: F) -> Self {
				Self { func, options }
			}
		}

		impl<F: FnMut(&mut $node_type)> RsxVisitorMut for $visitor_name<F> {
			fn ignore_element_children(&self) -> bool {
				self.options.ignore_element_children
			}

			fn ignore_component_node(&self) -> bool {
				self.options.ignore_component_node
			}

			fn ignore_component_slot_children(&self) -> bool {
				self.options.ignore_component_slot_children
			}

			fn $visit_method(&mut self, value: &mut $node_type) {
				(self.func)(value);
			}
		}
	};
}



impl_visitor!(VisitRsxNode, RsxNode, visit_node);
impl_visitor!(VisitRsxElement, RsxElement, visit_element);
impl_visitor!(VisitRsxComponent, RsxComponent, visit_component);
impl_visitor!(VisitRsxText, str, visit_text);
impl_visitor!(VisitRsxComment, str, visit_comment);

impl_visitor_mut!(VisitRsxNodeMut, RsxNode, visit_node);
impl_visitor_mut!(VisitRsxElementMut, RsxElement, visit_element);
impl_visitor_mut!(VisitRsxComponentMut, RsxComponent, visit_component);
impl_visitor_mut!(VisitRsxTextMut, str, visit_text);
impl_visitor_mut!(VisitRsxCommentMut, str, visit_comment);




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut count = 0;

		VisitRsxNodeMut::new(|_| count += 1)
			.walk_node(&mut rsx! { <div /> }.node);
		expect(count).to_be(1);
	}
}

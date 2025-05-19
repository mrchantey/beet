use crate::prelude::*;
use beet_common::prelude::*;
use rapidhash::RapidHashSet;

/// Find all [`WebDirective::StyleId`] directives and convert them
/// to class names. These are usually applied soley by the [`LangTemplateMap`]
pub struct ApplyStyleIds {
	/// the attribute to use as a selector for the component,
	/// defaults to "data-styleid".
	/// This must match the one used in [`ParseComponentStyles`]
	attr: String,
	/// Elements are not visible and so should not have a style id
	ignored_tags: Vec<String>,
}

impl Default for ApplyStyleIds {
	fn default() -> Self {
		ApplyStyleIds {
			attr: Self::DEFAULT_STYLEID_ATTR.to_string(),
			ignored_tags: vec![
				"style".to_string(),
				"script".to_string(),
				"link".to_string(),
				"meta".to_string(),
				"head".to_string(),
			],
		}
	}
}

impl Pipeline<WebNode, WebNode> for ApplyStyleIds {
	/// Applies scoped style to:
	/// 1. root node
	/// 2. all component nodes
	/// 3. all component slot children
	fn apply(mut self, mut node: WebNode) -> WebNode {
		// 1. apply to the root node, if its a component nothing happens
		//    in this step, it will be handled by the component visitor
		self.apply_node(&mut node);

		// visit all components
		VisitRsxComponentMut::walk(&mut node, |component| {
			// 2. apply to component node
			self.apply_node(&mut component.node);
		});
		node
	}
}

impl ApplyStyleIds {
	pub const DEFAULT_STYLEID_ATTR: &'static str = "data-styleid";

	/// a class name in the format `data-styleid-0`,
	/// this allows for multiple classes on a single element,
	/// which is required for style:cascade
	fn class_name(&self, id: u64) -> String { format!("{}-{}", self.attr, id) }

	/// 1. apply the idx to all style bodies
	/// 2. if contains style, apply tag to all elements in the component
	fn apply_node(&mut self, node: &mut WebNode) {
		// the boundary for scoped style is to apply to every descendent
		// with the exception of component nodes
		let mut ids = RapidHashSet::default();

		// 1. collect all ids in this component
		VisitWebNodeMut::walk_with_opts(
			node,
			VisitRsxOptions::ignore_component_node(),
			|node| {
				if let Some(id) = node.style_id() {
					ids.insert(id);
					// remove the node
					*node = Default::default();
				}
			},
		);

		// 2. tag elements in this component with the style ids
		// Components tagged as cascade will also be traversed
		if !ids.is_empty() {
			VisitRsxElementMut::walk_with_opts(
				node,
				// opts.clone(),
				VisitRsxOptions::should_visit_component_node(|c| {
					c.is_cascade_style()
				}),
				|el| {
					if self.ignored_tags.contains(&el.tag) {
						return;
					}
					for id in ids.iter() {
						let class_name = self.class_name(*id);
						el.attributes.push(RsxAttribute::Key {
							key: class_name.clone(),
						});
					}
				},
			);
		}
	}
}

#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[derive(Node)]
	struct JustSlot;

	fn just_slot(_: JustSlot) -> WebNode {
		rsx! { <slot /> }
	}
	#[derive(Node)]
	struct Child;

	fn child(_: Child) -> WebNode {
		rsx! {
			<div>
				<style style:id=8 />
				<slot />
			</div>
		}
	}

	#[test]
	fn removes_element() {
		rsx! {<style style:id=7 />}
			.xpipe(ApplyStyleIds::default())
			.xmap(|n| n.is_empty())
			.xpect()
			.to_be_true();
	}
	#[test]
	fn root() {
		rsx! {
			<div>
			<style style:id=7 />
			foobar
			</div>
		}
		.xpipe(ApplyStyleIds::default())
		.xpipe(RsxToHtmlString::default().trim())
		.unwrap()
		.xpect()
		.to_be("<div data-styleid-7>foobar</div>");
	}
	#[test]
	fn deduplicates() {
		rsx! {
			<div>
			<style style:id=7 />
			<style style:id=7 />
			foobar
			</div>
		}
		.xpipe(ApplyStyleIds::default())
		.xpipe(RsxToHtmlString::default().trim())
		.unwrap()
		.xpect()
		.to_be("<div data-styleid-7>foobar</div>");
	}

	#[test]
	fn component() {
		rsx! { <Child /> }
			.xpipe(ApplyStyleIds::default())
			.xpipe(RsxToHtmlString::default().trim())
			.unwrap()
			.xpect()
			.to_be("<div data-styleid-8></div>");
	}

	#[test]
	fn nested_component() {
		rsx! {
			<Child>
				<Child />
			</Child>
		}
		.xpipe(ApplyStyleIds::default())
		.xpipe(RsxToHtmlString::default().trim())
		.unwrap()
		.xpect()
		.to_be("<div data-styleid-8><div data-styleid-8></div></div>");
	}

	#[test]
	fn slot_children() {
		rsx! {
			<JustSlot>
				<br/>
			</JustSlot>
			<style style:id=9 />
		}
		.xpipe(ApplyStyleIds::default())
		.xpipe(RsxToHtmlString::default().trim())
		.unwrap()
		.xpect()
		.to_be("<br data-styleid-9/>");
	}
	#[test]
	fn style_cascade() {
		rsx! {
			<Child style:cascade/>
			<style style:id="2" />
		}
		.xpipe(ApplyStyleIds::default())
		.xpipe(RsxToHtmlString::default().trim())
		.unwrap()
		.xpect()
		.to_be("<div data-styleid-2 data-styleid-8></div>");
	}
}

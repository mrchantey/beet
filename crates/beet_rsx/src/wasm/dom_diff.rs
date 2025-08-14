use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;

#[derive(SystemParam)]
pub(crate) struct DomDiff<'w, 's> {
	fragments: Query<'w, 's, (), With<FragmentNode>>,
	elements: Query<'w, 's, &'static NodeTag, With<ElementNode>>,
	text_nodes: Query<'w, 's, &'static mut TextNode>,
	children: Query<'w, 's, &'static Children>,
	constants: Res<'w, HtmlConstants>,
}

impl DomDiff<'_, '_> {
	pub fn diff_node(
		&mut self,
		entity: Entity,
		parent: &web_sys::HtmlElement,
		node: web_sys::Node,
	) -> Result {
		if self.elements.contains(entity) {
			match node.dyn_into::<web_sys::HtmlElement>() {
				Ok(element) => {
					self.diff_element(entity, element)?;
				}
				Err(node) => {
					self.remove_node(parent, node)?;
					self.create_node(parent, entity)?;
				}
			}
		} else if self.text_nodes.contains(entity) {
			match node.dyn_into::<web_sys::Text>() {
				Ok(text) => {
					self.diff_text(entity, text)?;
				}
				Err(node) => {
					self.remove_node(parent, node)?;
					self.create_node(parent, entity)?;
				}
			}
		}
		Ok(())
	}

	/// Apply a diff to element tag attributes and children
	pub fn diff_element(
		&mut self,
		entity: Entity,
		element: web_sys::HtmlElement,
	) -> Result {
		todo!();
	}
	pub fn diff_text(
		&mut self,
		entity: Entity,
		dom_text: web_sys::Text,
	) -> Result {
		let entity_text = self.text_nodes.get_mut(entity)?;
		if entity_text.0 != dom_text.data() {
			dom_text.set_data(&entity_text.0);
		}
		Ok(())
	}

	/// Apply a diff to children, ignoring element tag and attributes
	pub fn diff_children(
		&mut self,
		entity: Entity,
		element: web_sys::HtmlElement,
	) -> Result {
		let dom_children = element.child_nodes();
		let mut entity_children = Vec::new();
		self.child_nodes(entity, &mut entity_children);

		let num_dom_children = dom_children.length() as usize;
		let num_entity_children = entity_children.len();
		for index in 0..usize::max(num_dom_children, num_entity_children) {
			match (index < num_dom_children, index < num_entity_children) {
				(true, true) => {
					let dom_child = dom_children.get(index as u32).unwrap();
					let entity_child = entity_children.get(index).unwrap();
					self.diff_node(*entity_child, &element, dom_child)?;
				}
				(true, false) => {
					// we still have dom nodes but no more entity nodes, remove them
					for index in index..num_dom_children {
						let dom_child = dom_children.get(index as u32).unwrap();
						self.remove_node(&element, dom_child.clone())?;
					}
				}
				(false, true) => {
					// we still have entity nodes but no more dom nodes, add them
					for index in index..num_entity_children {
						let entity_child = entity_children.get(index).unwrap();
						self.create_node(&element, *entity_child)?;
					}
				}
				(false, false) => {
					unreachable!("outer loop prevents this")
				}
			}
		}
		Ok(())
	}
	/// Get element and text child nodes, flattening fragments
	fn child_nodes(&self, entity: Entity, children: &mut Vec<Entity>) {
		for child in self.children.iter_direct_descendants(entity) {
			if self.elements.contains(child) {
				children.push(child);
			} else if self.fragments.contains(child) {
				self.child_nodes(child, children);
			}
		}
	}

	fn create_node(
		&mut self,
		parent: &web_sys::HtmlElement,
		entity: Entity,
	) -> Result {
		todo!("create node");
	}
	fn remove_node(
		&mut self,
		parent: &web_sys::HtmlElement,
		node: web_sys::Node,
	) -> Result<()> {
		parent.remove_child(&node).map_jserr()?;
		Ok(())
	}
}

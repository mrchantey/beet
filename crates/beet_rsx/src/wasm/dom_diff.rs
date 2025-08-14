use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;

#[derive(SystemParam)]
pub(crate) struct DomDiff<'w, 's> {
	commands: Commands<'w, 's>,
	fragment_nodes: Query<'w, 's, (), With<FragmentNode>>,
	element_nodes: Query<
		'w,
		's,
		(&'static NodeTag, Option<&'static InnerText>),
		With<ElementNode>,
	>,
	doctype_nodes: Query<'w, 's, (), With<DoctypeNode>>,
	comment_nodes: Query<'w, 's, &'static CommentNode>,
	text_nodes: Query<'w, 's, &'static mut TextNode, Without<AttributeOf>>,
	children: Query<'w, 's, &'static Children>,
	constants: Res<'w, HtmlConstants>,
	attributes: Query<'w, 's, &'static Attributes>,
	attribute_nodes: Query<
		'w,
		's,
		(
			&'static AttributeKey,
			Option<&'static TextNode>,
			Option<&'static NumberNode>,
			Option<&'static BoolNode>,
		),
		With<AttributeOf>,
	>,
}

impl DomDiff<'_, '_> {
	pub fn diff_node(
		&mut self,
		entity: Entity,
		parent: &web_sys::HtmlElement,
		node: web_sys::Node,
	) -> Result {
		if self.element_nodes.contains(entity) {
			match node.dyn_into::<web_sys::HtmlElement>() {
				Ok(element) => {
					self.diff_element(entity, element)?;
				}
				Err(node) => {
					self.remove_node(parent, node)?;
					self.append_node(parent, entity)?;
				}
			}
		} else if self.text_nodes.contains(entity) {
			match node.dyn_into::<web_sys::Text>() {
				Ok(text) => {
					self.diff_text(entity, text)?;
				}
				Err(node) => {
					self.remove_node(parent, node)?;
					self.append_node(parent, entity)?;
				}
			}
		}
		Ok(())
	}

	/// Apply a diff to element tag attributes and children
	///
	/// ## Errors
	/// Errors if the element does not have a parent
	pub fn diff_element(
		&mut self,
		entity: Entity,
		element: web_sys::HtmlElement,
	) -> Result {
		let (node_tag, inner_text) = self.element_nodes.get(entity)?;

		if **node_tag == element.tag_name() {
			// tags match, perform diff
			if let Some(inner_text) = inner_text
				&& element.inner_text() != **inner_text
			{
				element.set_inner_text(&*inner_text);
			}
			self.diff_attributes(entity, element.clone())?;
			self.diff_children(entity, element)?;
		} else {
			// tag name mismatch, remove and append
			let parent = element.parent_element().ok_or_else(|| {
				bevyhow!("DomDiff: Cannot diff an element without a parent")
			})?;
			self.remove_node(
				&parent,
				element.dyn_into::<web_sys::Node>().unwrap(),
			)?;
			self.append_node(&parent, entity)?;
		}


		Ok(())
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
		let entity_children = self.child_nodes(entity);

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
					break;
				}
				(false, true) => {
					// we still have entity nodes but no more dom nodes, add them
					for index in index..num_entity_children {
						beet_utils::log!("appending node at index {index}");
						let entity_child = entity_children.get(index).unwrap();
						self.append_node(&element, *entity_child)?;
					}
					break;
				}
				(false, false) => {
					unreachable!("outer loop prevents this")
				}
			}
		}
		Ok(())
	}


	pub fn diff_attributes(
		&mut self,
		entity: Entity,
		element: web_sys::HtmlElement,
	) -> Result {
		let el_attributes = element.get_attribute_names();
		let entity_attributes = self
			.attributes
			.get(entity)
			.map(|a| {
				a.iter()
					.filter_map(|a| self.attribute_nodes.get(a).ok())
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();

		let num_dom_attributes = el_attributes.length() as usize;
		let num_entity_attributes = entity_attributes.len();
		for index in 0..usize::max(num_dom_attributes, num_entity_attributes) {
			match (index < num_dom_attributes, index < num_entity_attributes) {
				(true, true) => {
					let dom_attr_name =
						el_attributes.get(index as u32).as_string().unwrap();
					let (entity_attr_name, text, _, _) =
						entity_attributes.get(index).unwrap();
					if dom_attr_name == entity_attr_name.0 {
						match (*text, element.get_attribute(&dom_attr_name)) {
							(None, Some(_)) => {
								element
									.set_attribute(&dom_attr_name, "")
									.map_jserr()?;
							}
							(Some(val), None) => {
								element
									.set_attribute(&dom_attr_name, val)
									.map_jserr()?;
							}
							(Some(val), Some(el_val)) if ***val != el_val => {
								element
									.set_attribute(&dom_attr_name, val)
									.map_jserr()?;
							}
							(Some(_), Some(_)) => { /*match */ }
							(None, None) => {}
						}
					} else {
					}
				}
				(true, false) => {
					// we still have dom attrs but no more entity attrs, remove them
					for index in index..num_dom_attributes {
						let key = el_attributes
							.get(index as u32)
							.as_string()
							.unwrap();
						element.remove_attribute(&key).map_jserr()?;
					}
					break;
				}
				(false, true) => {
					// we still have entity attrs but no more dom attrs, add them
					for index in index..num_entity_attributes {
						beet_utils::log!("appending node at index {index}");
						let (key, text, _, _) =
							entity_attributes.get(index).unwrap();
						let text = text.map(|t| t.0.as_ref()).unwrap_or("");
						element.set_attribute(key, text).map_jserr()?;
					}
					break;
				}
				(false, false) => {
					unreachable!("outer loop prevents this")
				}
			}
		}
		Ok(())
	}

	/// Get element and text child nodes, flattening fragments
	fn child_nodes(&self, entity: Entity) -> Vec<Entity> {
		fn collect_children(
			this: &DomDiff,
			entity: Entity,
			out: &mut Vec<Entity>,
		) {
			for child in this.children.iter_direct_descendants(entity) {
				if this.element_nodes.contains(child)
					|| this.text_nodes.contains(child)
					|| this.doctype_nodes.contains(child)
					|| this.comment_nodes.contains(child)
				{
					out.push(child);
				} else if this.fragment_nodes.contains(child) {
					collect_children(this, child, out);
				}
			}
		}
		let mut children = Vec::new();
		collect_children(self, entity, &mut children);
		children
	}

	fn append_node(
		&mut self,
		parent: &web_sys::Element,
		entity: Entity,
	) -> Result<web_sys::Node> {
		if let Ok((node_tag, inner_text)) = self.element_nodes.get(entity) {
			let node = web_sys::window()
				.unwrap()
				.document()
				.unwrap()
				.create_element(&node_tag)
				.unwrap();
			let node = node.dyn_into::<web_sys::HtmlElement>().unwrap();
			if let Some(inner_text) = inner_text {
				node.set_inner_text(&inner_text.0);
			}
			parent.append_child(&node).unwrap();
			self.commands
				.entity(entity)
				.insert(DomElementBinding::new(node.clone()));
			self.diff_attributes(entity, node.clone())?;
			for child in self.child_nodes(entity) {
				self.append_node(&node, child)?;
			}
			Ok(node.dyn_into::<web_sys::Node>().unwrap())
		} else if let Ok(text) = self.text_nodes.get(entity) {
			let node = web_sys::window()
				.unwrap()
				.document()
				.unwrap()
				.create_text_node(&text.0);
			parent.append_child(&node).unwrap();
			self.commands
				.entity(entity)
				.insert(DomTextBinding::new(node.clone()));
			Ok(node.dyn_into::<web_sys::Node>().unwrap())
		} else {
			bevybail!("entity is neither element nor text")
		}
	}
	fn remove_node(
		&mut self,
		parent: &web_sys::Element,
		node: web_sys::Node,
	) -> Result<()> {
		parent.remove_child(&node).map_jserr()?;
		Ok(())
	}
}

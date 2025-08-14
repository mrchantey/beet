use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

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
	requires_node_binding:
		Query<'w, 's, Entity, (With<SignalEffect>, Without<DomNodeBinding>)>,
}

impl DomDiff<'_, '_> {
	/// Appends the node to its parent then performs a [`Self::diff_node`]
	pub fn append_node(
		&mut self,
		parent: &web_sys::Element,
		entity: Entity,
	) -> Result<web_sys::Node> {
		let node = if let Ok((tag, _)) = self.element_nodes.get(entity) {
			web_sys::window()
				.unwrap()
				.document()
				.unwrap()
				.create_element(&tag)
				.unwrap()
				.into()
		} else if let Ok(text) = self.text_nodes.get(entity) {
			web_sys::window()
				.unwrap()
				.document()
				.unwrap()
				.create_text_node(&text.0)
				.into()
		} else if let Ok(comment) = self.comment_nodes.get(entity) {
			web_sys::window()
				.unwrap()
				.document()
				.unwrap()
				.create_comment(&**comment)
				.into()
		} else if let Ok(_) = self.doctype_nodes.get(entity) {
			todo!("create doctype?");
		} else {
			bevybail!("entity is not a node")
		};
		parent.append_child(&node).unwrap();
		self.diff_node(entity, parent, &node)?;
		Ok(node)
	}

	fn remove_node(
		&mut self,
		parent: &web_sys::Element,
		node: &web_sys::Node,
	) -> Result<()> {
		parent.remove_child(&node).map_jserr()?;
		Ok(())
	}
	pub fn diff_node(
		&mut self,
		entity: Entity,
		parent: &web_sys::Element,
		node: &web_sys::Node,
	) -> Result {
		let node = if self.element_nodes.contains(entity) {
			match node.dyn_ref::<web_sys::Element>() {
				Some(element) => {
					self.diff_element(entity, element)?;
					node.clone()
				}
				None => {
					self.remove_node(parent, node)?;
					self.append_node(parent, entity)?
				}
			}
		} else if let Ok(entity_text) = self.text_nodes.get(entity) {
			match node.dyn_ref::<web_sys::Text>() {
				Some(dom_text) => {
					if entity_text.0 != dom_text.data() {
						dom_text.set_data(&entity_text.0);
					}
					node.clone()
				}
				None => {
					self.remove_node(parent, node)?;
					self.append_node(parent, entity)?
				}
			}
		} else if let Ok(entity_comment) = self.comment_nodes.get(entity) {
			match node.dyn_ref::<web_sys::Comment>() {
				Some(dom_comment) => {
					if entity_comment.0 != dom_comment.data() {
						dom_comment.set_data(&entity_comment.0);
					}
					node.clone()
				}
				None => {
					self.remove_node(parent, node)?;
					self.append_node(parent, entity)?
				}
			}
		} else {
			node.clone()
		};
		if self.requires_node_binding.contains(entity) {
			self.commands
				.entity(entity)
				.insert(DomNodeBinding::new(node));
		}

		Ok(())
	}

	/// Apply a diff to element tag attributes and children
	///
	/// ## Errors
	/// - The entity is not an [`ElementNode`]
	/// - The tags dont match and the element does not have a parent
	fn diff_element(
		&mut self,
		entity: Entity,
		element: &web_sys::Element,
	) -> Result {
		let (node_tag, inner_text) = self.element_nodes.get(entity)?;

		if node_tag.to_lowercase() == element.tag_name().to_lowercase() {
			// tags match, perform diff
			// 1. inner text
			if let Some(inner_text) = inner_text {
				let html_el =
					element.dyn_ref::<HtmlElement>().ok_or_else(|| {
						bevyhow!(
							"Entity has an InnerText but element is not a HtmlElement"
						)
					})?;
				if html_el.inner_text() != **inner_text {
					html_el.set_inner_text(&*inner_text);
				}
			}
			// 2. attributes
			self.diff_attributes(entity, element)?;
			// 3. children
			self.diff_children(entity, element)?;
		} else {
			// tag name mismatch, remove and append
			let parent = element.parent_element().ok_or_else(|| {
				bevyhow!("DomDiff: Cannot diff an element without a parent")
			})?;
			self.remove_node(
				&parent,
				element.dyn_ref::<web_sys::Node>().unwrap(),
			)?;
			// caution! this could result in infinite loop if appended element
			// doesnt have matching tag name
			self.append_node(&parent, entity)?;
		}


		Ok(())
	}

	/// Apply a diff to children, ignoring element tag and attributes
	pub fn diff_children(
		&mut self,
		entity: Entity,
		element: &web_sys::Element,
	) -> Result {
		let dom_children = element.child_nodes();
		let entity_children = self.child_nodes(entity);

		// Phase 1: iterate entity children, update existing DOM child or append if missing
		let num_dom_children = dom_children.length() as usize;
		for index in 0..entity_children.len() {
			let entity_child = entity_children[index];
			if index < num_dom_children {
				let dom_child = dom_children.get(index as u32).unwrap();
				self.diff_node(entity_child, &element, &dom_child)?;
			} else {
				self.append_node(&element, entity_child)?;
			}
		}

		// Phase 2: remove any extra DOM children beyond the number of entity children
		let num_dom_children = dom_children.length() as usize;
		if num_dom_children > entity_children.len() {
			for index in (entity_children.len()..num_dom_children).rev() {
				let dom_child = dom_children.get(index as u32).unwrap();
				self.remove_node(&element, &dom_child)?;
			}
		}
		Ok(())
	}


	pub fn diff_attributes(
		&mut self,
		entity: Entity,
		element: &web_sys::Element,
	) -> Result {
		let el_attributes = element.get_attribute_names();
		let entity_attributes = self
			.attributes
			.iter_direct_descendants(entity)
			.filter_map(|a| self.attribute_nodes.get(a).ok())
			.collect::<Vec<_>>();

		// 1: ensure all entity attributes exist in DOM with correct values
		for (key, text, _, _) in &entity_attributes {
			let desired = text.map(|t| t.0.as_ref()).unwrap_or("");
			match element.get_attribute(&key.0) {
				Some(current) => {
					if current != desired {
						element.set_attribute(key, desired).map_jserr()?;
					}
				}
				None => {
					element.set_attribute(key, desired).map_jserr()?;
				}
			}
		}

		// 2: remove any DOM attributes not present in entity
		let num_dom_attributes = el_attributes.length() as usize;
		for index in (0..num_dom_attributes).rev() {
			let name = el_attributes.get(index as u32).as_string().unwrap();
			let exists =
				entity_attributes.iter().any(|(k, _, _, _)| k.0 == name);
			if !exists {
				element.remove_attribute(&name).map_jserr()?;
			}
		}

		// 3. ensure dom node bindings
		for attr in self.attributes.iter_direct_descendants(entity) {
			if self.requires_node_binding.contains(attr) {
				self.commands
					.entity(attr)
					.insert(DomNodeBinding::new(element.clone()));
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
}

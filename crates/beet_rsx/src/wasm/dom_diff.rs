use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

use crate::wasm::DomNodeBinding;

#[derive(SystemParam)]
pub struct DomDiff<'w, 's> {
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
	attributes: Query<'w, 's, &'static Attributes>,
	constants: Res<'w, HtmlConstants>,
	attribute_nodes: Query<
		'w,
		's,
		(&'static AttributeKey, Option<&'static TextNode>),
		With<AttributeOf>,
	>,
	dom_node_bindings: Query<'w, 's, &'static DomNodeBinding>,
}

impl DomDiff<'_, '_> {
	/// Appends the node to its parent then performs a [`Self::diff_node`]
	pub fn append_node(
		&mut self,
		parent: &web_sys::Element,
		entity: Entity,
	) -> Result<web_sys::Node> {
		let node = self.create_node(parent, entity)?;
		parent.append_child(&node).map_jserr()?;
		self.diff_node(entity, parent, &node)?;
		Ok(node)
	}

	fn create_node(
		&mut self,
		parent: &web_sys::Element,
		entity: Entity,
	) -> Result<web_sys::Node> {
		let document = web_sys::window()
			.ok_or_else(|| bevyhow!("no window"))?
			.document()
			.ok_or_else(|| bevyhow!("no document"))?;
		let node = if let Ok((tag, _)) = self.element_nodes.get(entity) {
			// check which namespace we're in and apply it
			// TODO foreignObject
			let ns = parent.namespace_uri();
			let node: web_sys::Node = match ns.as_deref() {
				Some("http://www.w3.org/2000/svg") => document
					.create_element_ns(Some("http://www.w3.org/2000/svg"), &tag)
					.map_jserr()?
					.into(),
				Some("http://www.w3.org/1998/Math/MathML") => document
					.create_element_ns(
						Some("http://www.w3.org/1998/Math/MathML"),
						&tag,
					)
					.map_jserr()?
					.into(),
				_ => document.create_element(&tag).map_jserr()?.into(),
			};
			node
		} else if let Ok(text) = self.text_nodes.get(entity) {
			document.create_text_node(&text.0).into()
		} else if let Ok(comment) = self.comment_nodes.get(entity) {
			document.create_comment(&**comment).into()
		} else if let Ok(_) = self.doctype_nodes.get(entity) {
			todo!("create doctype?");
		} else {
			bevybail!("entity is not a node")
		};

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
		let mut needs_replace = false;

		if let Ok((node_tag, _)) = self.element_nodes.get(entity) {
			match node.dyn_ref::<web_sys::Element>() {
				Some(element) => {
					let ns = element.namespace_uri();
					let desired = node_tag.tag();
					let actual = element.tag_name();
					let tags_match = match ns.as_deref() {
						Some("http://www.w3.org/1999/xhtml") | None => {
							// html is not case sensitive, element.tag_name() often capitalized
							desired.eq_ignore_ascii_case(actual.as_str())
						}
						// other namespaces may be case sensitive
						_ => desired == actual.as_str(),
					};
					if tags_match {
						// tags match, perform diff
						self.diff_attributes(entity, element)?;
						self.diff_children(entity, element)?;
					} else {
						needs_replace = true;
					}
				}
				None => {
					needs_replace = true;
				}
			}
		} else if let Ok(entity_text) = self.text_nodes.get(entity) {
			match node.dyn_ref::<web_sys::Text>() {
				Some(dom_text) => {
					if entity_text.0 != dom_text.data() {
						dom_text.set_data(&entity_text.0);
					}
				}
				None => {
					needs_replace = true;
				}
			}
		} else if let Ok(entity_comment) = self.comment_nodes.get(entity) {
			match node.dyn_ref::<web_sys::Comment>() {
				Some(dom_comment) => {
					if entity_comment.0 != dom_comment.data() {
						dom_comment.set_data(&entity_comment.0);
					}
				}
				None => {
					needs_replace = true;
				}
			}
		}

		if needs_replace {
			let new_node = self.create_node(parent, entity)?;
			parent.replace_child(&new_node, node).map_jserr()?;
			// INFINITE LOOP:
			// should be Ok because guaranteed to match created node
			self.diff_node(entity, parent, &new_node)?;
		} else {
			self.diff_node_binding(entity, &node);
		}

		Ok(())
	}


	/// This diff is unusual because it modifies the entity, not the dom.
	/// for now just apply a DomNodeBinding to all elements and their attributes.
	/// in the future we could narrow this with something like RequiresDomBinding
	fn diff_node_binding(&mut self, entity: Entity, node: &web_sys::Node) {
		if let Ok(binding) = self.dom_node_bindings.get(entity)
			&& binding.nodes_eq(node)
		{
			// beet_utils::log!("diff node binding: {entity:?} match");
			return;
		} else {
			// beet_utils::log!("diff node binding: {entity:?} inserting");
			self.commands
				.entity(entity)
				.insert(DomNodeBinding::new(node.clone()));
		}
	}

	/// Apply a diff to children, the entity may be an [`ElementNode`] or [`FragmentNode`]
	pub fn diff_children(
		&mut self,
		entity: Entity,
		element: &web_sys::Element,
	) -> Result {
		// 1. check for InnerText, if so only do this check
		if let Ok((_, Some(inner_text))) = self.element_nodes.get(entity) {
			let html_el =
				element.dyn_ref::<HtmlElement>().ok_or_else(|| {
					bevyhow!(
						"Entity has an InnerText but element is not a HtmlElement"
					)
				})?;
			if html_el.inner_text() != **inner_text {
				html_el.set_inner_text(&*inner_text);
			}
			return Ok(());
		}

		// 2: iterate entity children, update existing DOM child or append if missing
		let entity_children = self.child_nodes(entity);
		let dom_children = {
			// NodeList to Vec
			let node_list = element.child_nodes();
			let mut dom_children =
				Vec::with_capacity(node_list.length() as usize);
			for i in 0..node_list.length() {
				if let Some(child) = node_list.item(i) {
					dom_children.push(child);
				}
			}
			dom_children
		};
		for index in 0..entity_children.len() {
			let entity_child = entity_children[index];
			if index < dom_children.len() {
				let dom_child = &dom_children[index];
				self.diff_node(entity_child, &element, dom_child)?;
			} else {
				self.append_node(&element, entity_child)?;
			}
		}

		// 3: remove any extra DOM children beyond the number of entity children
		if dom_children.len() > entity_children.len() {
			for index in (entity_children.len()..dom_children.len()).rev() {
				let dom_child = &dom_children[index];
				self.remove_node(&element, dom_child)?;
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
		for &(key, text) in &entity_attributes {
			let desired = text.map(|t| t.0.as_ref()).unwrap_or("").to_string();
			if desired == "false" {
				element.remove_attribute(&key.0).map_jserr()?;
				continue;
			}

			match element.get_attribute(&key.0) {
				Some(current) => {
					if current != desired {
						element.set_attribute(&key.0, &desired).map_jserr()?;
					}
				}
				None => {
					if key.is_event()
						&& let Some(text) = text
						&& text.contains(&self.constants.event_handler)
					{
						// ignore event handler attributes, we're already at runtime so
						// they will be bound directly to the dom in bind_events
					} else {
						element.set_attribute(&key.0, &desired).map_jserr()?;
					}
				}
			}
		}

		// 2: remove any DOM attributes not present in entity (using allow-list)
		let managed: std::collections::HashSet<String> =
			entity_attributes.iter().map(|(k, _)| k.0.clone()).collect();
		let num_dom_attributes = el_attributes.length() as usize;
		for index in (0..num_dom_attributes).rev() {
			let name = match el_attributes.get(index as u32).as_string() {
				Some(n) => n,
				None => continue,
			};
			// skip attributes that are present or protected
			if managed.contains(&name) {
				continue;
			}
			let is_protected = name == self.constants.dom_idx_key
				|| name == self.constants.style_id_key;
			let allowed_delete = !is_protected
				&& (name.starts_with("aria-")
					|| name.starts_with("data-")
					|| matches!(
						name.as_str(),
						"class"
							| "style" | "id" | "src"
							| "href" | "alt" | "title"
							| "type" | "name" | "placeholder"
							| "role" | "value" | "checked"
							| "disabled" | "selected"
							| "multiple" | "readonly"
							| "tabindex"
					));
			if allowed_delete {
				element.remove_attribute(&name).map_jserr()?;
			}
		}

		// 3. Diff node binding
		for attr in self.attributes.iter_direct_descendants(entity) {
			self.diff_node_binding(attr, &element);
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

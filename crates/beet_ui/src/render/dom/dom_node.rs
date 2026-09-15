//! [`DomNode`]: the binding from an entity to the node it paints as.
use crate::prelude::*;
use beet_core::exports::SendWrapper;
use beet_core::prelude::*;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

/// The DOM node an entity paints as: the binding the renderer makes on every
/// visit and the incremental pass patches through.
///
/// Three shapes, one per kind of entity in the walk: a node of the entity's
/// own (an element, a text node, a comment), the element an attribute entity
/// sets its attribute on, and a document element an entity binds in place
/// rather than creates (the render target a page's `<body>` stands in for).
/// The shape decides what a despawn takes with it: its node, its attribute,
/// or nothing.
///
/// A bound node carries its entity back as a property
/// ([`Self::ENTITY_PROPERTY`]), so an event's target resolves to its entity
/// by walking up the DOM ([`Self::entity_of`]).
#[derive(Debug, Component)]
#[component(on_remove = DomNode::on_remove)]
pub enum DomNode {
	/// A node created for the entity, removed with it.
	Node(SendWrapper<web_sys::Node>),
	/// The element an attribute entity sets its attribute on; the attribute
	/// is removed with the entity.
	Attribute(SendWrapper<web_sys::Element>),
	/// One of the document's own elements, bound in place and never removed.
	Document(SendWrapper<web_sys::Element>),
}

impl DomNode {
	/// The property a bound node carries its entity under, as the entity's
	/// bits.
	pub const ENTITY_PROPERTY: &str = "beetEntity";

	/// Bind to a node created for the entity.
	pub fn node(node: impl Into<web_sys::Node>) -> Self {
		Self::Node(SendWrapper::new(node.into()))
	}

	/// Bind an attribute entity to the element it sets its attribute on.
	pub fn attribute(element: web_sys::Element) -> Self {
		Self::Attribute(SendWrapper::new(element))
	}

	/// Bind to a document element in place.
	pub fn document(element: web_sys::Element) -> Self {
		Self::Document(SendWrapper::new(element))
	}

	/// The bound node, whichever shape.
	pub fn get(&self) -> &web_sys::Node {
		match self {
			Self::Node(node) => node,
			Self::Attribute(element) | Self::Document(element) => element,
		}
	}

	/// The element the entity paints as, if it paints as one: an element
	/// node of its own or a document element, never an attribute's.
	pub fn element(&self) -> Option<&web_sys::Element> {
		match self {
			Self::Node(node) => node.dyn_ref(),
			Self::Attribute(_) => None,
			Self::Document(element) => Some(element),
		}
	}

	/// Stamp `entity` onto `node`, so the node answers [`Self::entity_of`].
	pub fn stamp(node: &web_sys::Node, entity: Entity) {
		js_sys::Reflect::set(
			node,
			&JsValue::from_str(Self::ENTITY_PROPERTY),
			&JsValue::from(entity.to_bits()),
		)
		.ok();
	}

	/// The entity `node` paints, else the nearest ancestor's: how an event's
	/// target finds its entity.
	pub fn entity_of(node: &web_sys::Node) -> Option<Entity> {
		let key = JsValue::from_str(Self::ENTITY_PROPERTY);
		let mut current = Some(node.clone());
		while let Some(node) = current {
			if let Ok(bits) = js_sys::Reflect::get(&node, &key)
				&& let Ok(bits) = u64::try_from(bits)
				&& let Some(entity) = Entity::try_from_bits(bits)
			{
				return Some(entity);
			}
			current = node.parent_node();
		}
		None
	}

	/// Hook: a despawned entity takes its node or its attribute with it; a
	/// document element stays.
	fn on_remove(world: DeferredWorld, cx: HookContext) {
		let Some(binding) = world.get::<DomNode>(cx.entity) else {
			return;
		};
		match binding {
			Self::Node(node) => {
				if let Some(parent) = node.parent_node() {
					parent.remove_child(node).ok();
				}
			}
			Self::Attribute(element) => {
				let Some(name) = world
					.get::<Attribute>(cx.entity)
					.map(|attribute| attribute.to_string())
				else {
					return;
				};
				// the `class` attribute merges with the element's `Classes`,
				// which keep painting once the attribute is gone
				let classes = (name == "class")
					.then(|| world.get::<AttributeOf>(cx.entity))
					.flatten()
					.and_then(|of| world.get::<Classes>(**of))
					.and_then(|classes| {
						ElementView::join_classes(
							classes.iter().map(ClassName::as_selector),
						)
					});
				match classes {
					Some(class) => {
						element.set_attribute("class", &class).ok();
					}
					None => {
						element.remove_attribute(&name).ok();
					}
				}
			}
			Self::Document(_) => {}
		}
	}
}

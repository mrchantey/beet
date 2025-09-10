use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use bevy::ecs::component::HookContext;
use bevy::ecs::system::SystemParam;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;

#[derive(SystemParam)]
pub(crate) struct DomBinding<'w, 's> {
	commands: Commands<'w, 's>,
	elements: Query<'w, 's, &'static DomNodeBinding>,
	constants: Res<'w, HtmlConstants>,
}

impl DomBinding<'_, '_> {
	/// Returns the element bound to this entity or finds the element in the dom
	/// with the provided [`DomIdx`] and insert a [`DomElementBinding`]
	pub fn get_or_bind_element(
		&mut self,
		entity: Entity,
		idx: DomIdx,
	) -> Result<web_sys::Element> {
		if let Ok(binding) = self.elements.get(entity) {
			return binding
				.inner()
				.clone()
				.dyn_into::<web_sys::Element>()
				.map_err(|node| bevyhow!("node is not an element {node:?}"));
		}
		let query =
			format!("[{}='{}']", self.constants.dom_idx_key, idx.inner());
		if let Some(el) = web_sys::window()
			.unwrap()
			.document()
			.unwrap()
			.query_selector(&query)
			.unwrap()
		{
			self.commands
				.entity(entity)
				.insert(DomNodeBinding::new(el.clone()));
			return Ok(el);
		} else {
			return Err(format!(
				"Element with DomIdx {} not found",
				idx.inner()
			)
			.into());
		}
	}
}


/// Bind an entity to a [`web_sys::Node`], added to each with a [`RequiresDomBinding`]
/// - [`TextNode`]
/// - [`CommentNode`]
/// - [`ElementNode`]
/// - [`AttributeOf`]
#[derive(Component, Deref, DerefMut)]
pub struct DomNodeBinding(SendWrapper<web_sys::Node>);
impl DomNodeBinding {
	pub fn new(node: impl Into<web_sys::Node>) -> Self {
		Self(SendWrapper::new(node.into()))
	}
	pub fn inner(&self) -> &web_sys::Node { self.0.as_ref() }
	/// Performs a strict equality `js ===` on two nodes, ensuring
	/// they point to the same instance
	pub fn nodes_eq(&self, other: &web_sys::Node) -> bool { &*self.0 == other }
}


/// Bind each [`ElementNode`] with a [`RequiresDomBinding`] and [`DomIdx`]
pub(crate) fn bind_dom_idx_elements(
	query: Populated<
		(Entity, &DomIdx),
		(
			With<ElementNode>,
			With<RequiresDomBinding>,
			Without<DomNodeBinding>,
		),
	>,
	mut binding: DomBinding,
) -> Result<()> {
	for (entity, dom_idx) in query.iter() {
		binding.get_or_bind_element(entity, *dom_idx)?;
	}
	Ok(())
}

/// Attach the text nodes to the DOM with the following steps:
/// 1. find the parent element of the text node
/// 2. find the marker comment node
/// 3. assign the text node to the next sibling of the marker comment
/// 4. remove the marker comments
pub(crate) fn bind_dom_idx_text_nodes(
	query: Populated<
		(Entity, &DomIdx),
		(
			Or<(With<TextNode>, With<CommentNode>)>,
			With<RequiresDomBinding>,
			Without<DomNodeBinding>,
			Without<AttributeOf>,
		),
	>,
	mut commands: Commands,
	constants: Res<HtmlConstants>,
	mut binding: DomBinding,
	parents: Query<&ChildOf>,
	elements: Query<(Entity, &DomIdx), With<ElementNode>>,
) -> Result<()> {
	for (entity, dom_idx) in query.iter() {
		// 1. get the parent element
		let Some((parent_entity, parent_idx)) = parents
			.iter_ancestors(entity)
			.find_map(|ancestor| elements.get(ancestor).ok())
		else {
			return Err(format!(
				"Reactive TextNode with {dom_idx} with has no parent ElementNode"
			)
			.into());
		};

		let element =
			binding.get_or_bind_element(parent_entity, *parent_idx)?;
		let children = element.child_nodes();

		// 2. find the marker comment node
		let expected_data =
			format!("{}|{}", constants.text_node_marker, dom_idx.0);

		let mut comment_idx = None;
		for i in 0..children.length() {
			let child = children.item(i).unwrap();
			if child.node_type() == web_sys::Node::COMMENT_NODE
				&& child.node_value().as_deref() == Some(&expected_data)
			{
				comment_idx = Some(i);
				break;
			}
		}

		let Some(comment_idx) = comment_idx else {
			return Err(format!(
				"Could not find marker comment for text node {dom_idx}"
			)
			.into());
		};

		// 3. assign the DomTextBinding
		let node = children.item(comment_idx + 1).ok_or_else(|| {
			format!("No text node after marker comment for {dom_idx}")
		})?;

		commands.entity(entity).insert(DomNodeBinding::new(node));

		// 4. remove marker comments
		// im not sure about this, if somebody calls normalize() we lose our text nodes
		// children.item(comment_idx + 2).map(|after| {
		// 	element.remove_child(&after).ok();
		// });
		// children.item(comment_idx).map(|comment| {
		// 	element.remove_child(&comment).ok();
		// });
	}
	Ok(())
}



pub(crate) fn bind_dom_idx_attributes(
	mut commands: Commands,
	mut dom_binding: DomBinding,
	elements: Query<(Entity, &DomIdx)>,
	query: Populated<
		(Entity, &AttributeOf),
		(
			Changed<TextNode>,
			(With<RequiresDomBinding>, Without<DomNodeBinding>),
		),
	>,
) -> Result<()> {
	for (entity, parent) in query.iter() {
		let Ok((parent_entity, parent_idx)) = elements.get(parent.entity())
		else {
			return Err(format!(
				"AttributeOf {entity} has no parent with a DomIdx",
			)
			.into());
		};

		let element =
			dom_binding.get_or_bind_element(parent_entity, *parent_idx)?;
		commands.entity(entity).insert(DomNodeBinding::new(element));
	}
	Ok(())
}

/// Track a created closure to ensure it is not dropped
#[derive(Component)]
#[component(on_remove=on_remove_dom_closure)]
struct DomClosureBinding {
	event_name: String,
	element: SendWrapper<web_sys::Element>,
	closure:
		SendWrapper<wasm_bindgen::prelude::Closure<dyn FnMut(web_sys::Event)>>,
}

/// if the closure is an attribute event, ensure
fn on_remove_dom_closure(world: DeferredWorld, cx: HookContext) {
	if let Some(binding) = world.entity(cx.entity).get::<DomClosureBinding>() {
		binding
			.element
			.remove_event_listener_with_callback(
				&binding.event_name,
				binding.closure.as_ref().as_ref().unchecked_ref(),
			)
			.unwrap();
	}
}


pub(crate) fn bind_events(
	mut commands: Commands,
	query: Populated<
		(Entity, &DomNodeBinding),
		(With<EventTarget>, Added<DomNodeBinding>),
	>,
	find_attribute: FindAttribute,
) -> Result<()> {
	for (el_entity, binding) in query.iter() {
		// beet_utils::log!("binding event: {el_entity:?}");

		let Some(element) = binding.dyn_ref::<web_sys::Element>() else {
			bevybail!("DomNodeBinding with EventTarget is not an element")
		};

		for (attr_entity, attr_key) in find_attribute.events(el_entity) {
			let attr_key = attr_key.clone();
			let attr_key2 = attr_key.clone();
			// remove the temp event playback attribute
			element.remove_attribute(&attr_key).ok();

			let func = move |ev: web_sys::Event| {
				ReactiveApp::with(|app| {
					let mut commands = app.world_mut().commands();
					let mut commands = commands.entity(el_entity);
					DomEvent::trigger(&mut commands, &attr_key, ev);
					// apply commands
					app.world_mut().flush();
				})
			};
			let closure = Closure::wrap(Box::new(func) as Box<dyn FnMut(_)>);
			let event_name = attr_key2.replace("on", "");
			binding
				.add_event_listener_with_callback(
					&attr_key2.replace("on", ""),
					closure.as_ref().unchecked_ref(),
				)
				.unwrap();
			// closure.forget();
			commands.entity(attr_entity).insert(DomClosureBinding {
				event_name,
				closure: SendWrapper::new(closure),
				element: SendWrapper::new(element.clone()),
			});
		}
	}
	Ok(())
}

/// Apply a [`DomDiff`] for any changed [`SignalEffect`]
/// with a [`TextNode`] or [`ElementNode`]
pub(crate) fn update_dom_nodes(
	query: Populated<
		(Entity, &DomNodeBinding),
		(
			Changed<SignalEffect>,
			Or<(With<TextNode>, With<ElementNode>)>,
		),
	>,
	mut diff: DomDiff,
) -> Result {
	for (entity, binding) in query.iter() {
		let node = binding.inner().clone();
		let parent = node.parent_element().unwrap();
		diff.diff_node(entity, &parent, &node)?;
	}
	Ok(())
}

/// Apply a [`DomDiff`] for any changed [`SignalEffect`]
/// with a [`TextNode`] or [`ElementNode`]
pub(crate) fn update_fragments(
	query: Populated<Entity, (Changed<SignalEffect>, With<FragmentNode>)>,
	mut diff: DomDiff,
	parents: Query<&ChildOf>,
	elements: Query<(Entity, &DomNodeBinding)>,
) -> Result {
	for entity in query.iter() {
		// beet_utils::log!("updating fragments..");
		let (el_entity,el_binding) = parents.iter_ancestors(entity).find_map(|parent|{
			elements.get(parent).ok()
		}).ok_or_else(|| {
			bevyhow!(
				"FragmentNode with SignalEffect must have an ElementNode parent with DomNodeBinding\n
				was the DomIdx correctly applied to the ElementNode?"
			)
		})?;

		let node = el_binding.inner().clone();
		let parent = node.parent_element().unwrap();
		diff.diff_node(el_entity, &parent, &node)?;
	}
	Ok(())
}

/// The attributes of elements are applied in the render html step,
/// updating is applied to the DOM *properties* of the element
pub(crate) fn update_attribute_values(
	_: TempNonSendMarker,
	query: Populated<
		(
			&AttributeKey,
			&DomNodeBinding,
			&TextNode,
			Option<&NumberNode>,
			Option<&BoolNode>,
		),
		Changed<TextNode>,
	>,
) -> Result<()> {
	for (key, el, text, num, bool) in query.iter() {
		let value = if let Some(num) = num {
			wasm_bindgen::JsValue::from_f64(**num)
		} else if let Some(bool) = bool {
			wasm_bindgen::JsValue::from_bool(**bool)
		} else {
			wasm_bindgen::JsValue::from_str(&**text)
		};

		// el.set_attribute(&key.0, &value.to_string())
		// 	.map_err(|err| format!("{err:?}"))?;
		// TODO use heck for camelCase conversion
		js_sys::Reflect::set(
			el.inner().as_ref(),
			&wasm_bindgen::JsValue::from_str(&key.0),
			&value,
		)
		.map_err(|err| format!("{err:?}"))?;
	}
	Ok(())
}

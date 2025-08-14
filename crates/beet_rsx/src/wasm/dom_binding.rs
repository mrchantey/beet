use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use web_sys::HtmlElement;

#[derive(SystemParam)]
pub(crate) struct DomBinding<'w, 's> {
	commands: Commands<'w, 's>,
	elements: Query<'w, 's, &'static DomElementBinding>,
	constants: Res<'w, HtmlConstants>,
}

impl DomBinding<'_, '_> {
	/// Returns the element bound to this entity or finds the element in the dom
	/// with the provided [`DomIdx`] and insert a [`DomElementBinding`]
	pub fn get_or_bind_element(
		&mut self,
		entity: Entity,
		idx: DomIdx,
	) -> Result<HtmlElement> {
		if let Ok(binding) = self.elements.get(entity) {
			return Ok(binding.inner().clone());
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
			let el = el.dyn_into::<HtmlElement>().unwrap();

			self.commands
				.entity(entity)
				.insert(DomElementBinding::new(el.clone()));
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


/// Added to element entities and dynamic attribute entities
#[derive(Component, Deref)]
pub struct DomElementBinding(SendWrapper<HtmlElement>);
impl DomElementBinding {
	pub fn new(el: web_sys::HtmlElement) -> Self { Self(SendWrapper::new(el)) }
	pub fn inner(&self) -> &HtmlElement { self.0.as_ref() }
}

/// Binding to a DOM text node
#[derive(Component, Deref)]
pub struct DomTextBinding(SendWrapper<web_sys::Text>);
impl DomTextBinding {
	pub fn new(text: web_sys::Text) -> Self { Self(SendWrapper::new(text)) }
	pub fn inner(&self) -> &web_sys::Text { self.0.as_ref() }
}


/// Track a created closure to ensure it is not dropped
#[derive(Component, Deref)]
pub struct DomClosureBinding(
	SendWrapper<wasm_bindgen::prelude::Closure<dyn FnMut(web_sys::Event)>>,
);

pub(crate) fn bind_element_nodes(
	query: Populated<
		(Entity, &DomIdx),
		(
			With<ElementNode>,
			With<SignalEffect>,
			Without<DomElementBinding>,
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
pub(crate) fn bind_text_nodes(
	query: Populated<
		(Entity, &DomIdx),
		(
			With<TextNode>,
			With<SignalEffect>,
			Without<DomTextBinding>,
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
		let node = children
			.item(comment_idx + 1)
			.ok_or_else(|| {
				format!("No text node after marker comment for {dom_idx}")
			})?
			.dyn_into::<web_sys::Text>()
			.map_err(|_| {
				format!("Node after marker comment is not a TextNode")
			})?;

		commands.entity(entity).insert(DomTextBinding::new(node));

		// 4. remove marker comments
		// im not sure about this, its a bit of a vanity thing so it looks prettier in dev tools
		// but doing ths means if somebody calls normalize() we lose our text nodes
		children.item(comment_idx + 2).map(|after| {
			element.remove_child(&after).ok();
		});
		children.item(comment_idx).map(|comment| {
			element.remove_child(&comment).ok();
		});
	}
	Ok(())
}



pub(crate) fn bind_attribute_values(
	mut commands: Commands,
	mut get_binding: DomBinding,
	elements: Query<(Entity, &DomIdx)>,
	query: Populated<
		(Entity, &AttributeOf),
		(
			Changed<TextNode>,
			(With<SignalEffect>, Without<DomElementBinding>),
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
			get_binding.get_or_bind_element(parent_entity, *parent_idx)?;
		commands
			.entity(entity)
			.insert(DomElementBinding::new(element));
	}
	Ok(())
}


pub(crate) fn update_element_nodes(
	query: Populated<(Entity, &DomElementBinding), Changed<SignalEffect>>,
	mut diff: DomDiff,
) -> Result {
	for (entity, binding) in query.iter() {
		let el = binding.inner().clone();
		// let parent = el.parent_element().unwrap();
		diff.diff_element(entity, el)?;
	}
	Ok(())
}


pub(crate) fn update_text_nodes(
	_: TempNonSendMarker,
	query: Populated<(Entity, &DomTextBinding), Changed<SignalEffect>>,
	mut diff: DomDiff,
) -> Result<()> {
	for (entity, node) in query.iter() {
		diff.diff_text(entity, node.inner().clone())?;
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
			&DomElementBinding,
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

pub(crate) fn bind_events(
	mut commands: Commands,
	mut get_binding: DomBinding,
	query: Populated<
		(Entity, &DomIdx, &Attributes),
		(With<EventTarget>, Added<DomIdx>),
	>,
	attribute_query: Query<(Entity, &AttributeKey)>,
) -> Result<()> {
	for (el_entity, idx, attributes) in query.iter() {
		for (attr_entity, attr_key) in attributes
			.iter()
			.filter_map(|attr| attribute_query.get(attr).ok())
			.filter(|(_, key)| key.starts_with("on"))
		{
			let attr_key = attr_key.clone();
			let attr_key2 = attr_key.clone();
			let element = get_binding.get_or_bind_element(el_entity, *idx)?;
			// remove the temp event playback attribute
			element.remove_attribute(&attr_key).ok();

			let func = move |ev: web_sys::Event| {
				ReactiveApp::with(|app| {
					let mut commands = app.world_mut().commands();
					let mut commands = commands.entity(el_entity);
					BeetEvent::trigger(&mut commands, &attr_key, ev);
					// apply commands
					app.world_mut().flush();
					// we must update the app manually to flush any signals,
					// they will not be able to update the app themselves because
					// ReactiveApp is already borrowd
					app.update();
				})
			};

			let closure = Closure::wrap(Box::new(func) as Box<dyn FnMut(_)>);
			element
				.add_event_listener_with_callback(
					&attr_key2.replace("on", ""),
					closure.as_ref().unchecked_ref(),
				)
				.unwrap();
			commands
				.entity(attr_entity)
				.insert(DomClosureBinding(SendWrapper::new(closure)));
		}
	}
	Ok(())
}

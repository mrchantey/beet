use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;


/// Added to element entities and dynamic attribute entities
#[derive(Component, Deref)]
pub struct DomElementBinding(SendWrapper<web_sys::HtmlElement>);
impl DomElementBinding {
	pub fn inner(&self) -> &web_sys::HtmlElement { self.0.as_ref() }
}

/// Binding to a DOM text node
#[derive(Component, Deref)]
pub struct DomTextBinding(SendWrapper<web_sys::Text>);
impl DomTextBinding {
	pub fn inner(&self) -> &web_sys::Text { self.0.as_ref() }
}


/// Track a created closure, usually to ensure it is not dropped
#[derive(Component, Deref)]
pub struct DomClosureBinding(
	SendWrapper<wasm_bindgen::prelude::Closure<dyn FnMut(web_sys::Event)>>,
);
pub(super) fn update_text_nodes(
	_: TempNonSendMarker,
	query: Populated<(&TextNode, &DomTextBinding), Changed<TextNode>>,
) -> Result<()> {
	for (text, node) in query.iter() {
		node.set_data(text);
	}
	Ok(())
}


/// The attributes of elements are applied in the render html step,
/// updating is applied to the DOM *properties* of the element
pub(super) fn update_attribute_values(
	_: TempNonSendMarker,
	query: Populated<
		(&AttributeKey, &AttributeLit, &DomElementBinding),
		Changed<AttributeLit>,
	>,
) -> Result<()> {
	for (key, value, el) in query.iter() {
		// el.set_attribute(&key.0, &value.to_string())
		// 	.map_err(|err| format!("{err:?}"))?;
		// TODO use heck for camelCase conversion
		js_sys::Reflect::set(
			el.inner().as_ref(),
			&wasm_bindgen::JsValue::from_str(&key.0),
			&attribute_lit_to_js_value(value)?,
		)
		.map_err(|err| format!("{err:?}"))?;
	}
	Ok(())
}

fn attribute_lit_to_js_value(
	value: &AttributeLit,
) -> Result<wasm_bindgen::JsValue> {
	match value {
		AttributeLit::String(s) => Ok(wasm_bindgen::JsValue::from_str(s)),
		AttributeLit::Number(n) => Ok(wasm_bindgen::JsValue::from_f64(*n)),
		AttributeLit::Boolean(b) => Ok(wasm_bindgen::JsValue::from_bool(*b)),
	}
}

/// lazily uncollapse text nodes and bind to the DOM
pub(super) fn bind_text_nodes(
	mut commands: Commands,
	mut get_binding: GetDomBinding,
	parents: Query<&ChildOf>,
	elements: Query<(Entity, &DomIdx, &TextNodeParent)>,
	query: Populated<
		Entity,
		(
			Changed<TextNode>,
			(
				With<SignalReceiver<String>>,
				Without<DomTextBinding>,
				Without<AttributeOf>,
			),
		),
	>,
) -> Result<()> {
	for entity in query.iter() {
		// 1. get the parent element
		let Some((parent_entity, parent_idx, text_node_parent)) = parents
			.iter_ancestors(entity)
			.find_map(|ancestor| elements.get(ancestor).ok())
		else {
			return Err(format!(
				r#"
TextNode {entity} has no parent ElementNode
Please ensure that any text nodes are wrapped in an ElementNode:
✅ # Good: rsx!{{<div>{{my_signal}}</div>}}
❌ # Bad:	rsx!{{my_signal}}
"#,
			)
			.into());
		};

		let element = get_binding.get_element(parent_entity, *parent_idx)?;
		let children = element.child_nodes();

		// 2. uncollapse child text nodes
		for child in text_node_parent.text_nodes.iter() {
			// get the collapsed node
			let collapsed_text_node =
				children.item(child.child_index as u32).ok_or_else(|| {
					format!(
						"TextNodeParent {} has no child at index {}",
						parent_idx, child.child_index
					)
				})?;
			let mut current_node: web_sys::Text = collapsed_text_node
				.dyn_into()
				.map_err(|_| format!("Could not convert child to text node"))?;


			// iterate over the split positions and split the text node,
			// assigning the uncollapsed nodes to entities as required
			for (entity, position) in child
				.split_positions
				.iter()
				// dont split the last position
				.take(child.split_positions.len().saturating_sub(1))
			{
				// assign the text node to the entity if its dynamic
				if let Some(entity) = entity {
					commands.entity(*entity).insert(DomTextBinding(
						SendWrapper::new(current_node.clone()),
					));
				}
				//https://developer.mozilla.org/en-US/docs/Web/API/Text/splitText
				current_node =
					current_node.split_text(*position as u32).unwrap();
			}

			// handle the last entity
			if let Some(entity) =
				child.split_positions.last().and_then(|(entity, _)| *entity)
			{
				commands.entity(entity).insert(DomTextBinding(
					SendWrapper::new(current_node.clone()),
				));
			}
		}
	}
	Ok(())
}

pub(super) fn bind_attribute_values(
	mut commands: Commands,
	mut get_binding: GetDomBinding,
	elements: Query<(Entity, &DomIdx)>,
	query: Populated<
		(Entity, &AttributeOf),
		(
			Changed<AttributeLit>,
			(With<SignalReceiver<String>>, Without<DomElementBinding>),
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

		let element = get_binding.get_element(parent_entity, *parent_idx)?;
		commands
			.entity(entity)
			.insert(DomElementBinding(SendWrapper::new(element)));
	}
	Ok(())
}

pub(super) fn bind_events(
	mut commands: Commands,
	mut get_binding: GetDomBinding,
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
			let element = get_binding.get_element(el_entity, *idx)?;
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


#[derive(SystemParam)]
pub(super) struct GetDomBinding<'w, 's> {
	commands: Commands<'w, 's>,
	elements: Query<'w, 's, &'static DomElementBinding>,
	constants: Res<'w, HtmlConstants>,
}

impl GetDomBinding<'_, '_> {
	pub fn get_element(
		&mut self,
		entity: Entity,
		idx: DomIdx,
	) -> Result<web_sys::HtmlElement> {
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
			let el = el.dyn_into::<web_sys::HtmlElement>().unwrap();

			self.commands
				.entity(entity)
				.insert(DomElementBinding(SendWrapper::new(el.clone())));
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

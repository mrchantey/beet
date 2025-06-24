use crate::prelude::*;
use beet_common::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use js_sys::Array;
use js_sys::Reflect;
use wasm_bindgen::JsCast;
use web_sys::Event;
use beet_bevy::prelude::*;

/// A system that runs once after hydration to playback any events
/// that occured while the wasm was loading.
pub(super) fn event_playback(
	constants: Res<HtmlConstants>,
	mut commands: Commands,
	query: Populated<(Entity, &TreeIdx, &EventKey), Added<TreeIdx>>,
) -> Result<()> {
	let event_map: HashMap<(TreeIdx, &EventKey), Entity> = query
		.iter()
		.map(|(entity, idx, event)| ((*idx, event), entity))
		.collect();


	beet_global_js::GLOBAL.with(|global| {
		let event_store =
			Reflect::get(&global, &constants.event_store.clone().into())
				.map_err(|_| {
					format!(
						"could not find event store: 'globalThis.{}'",
						constants.event_store
					)
				})?;
		if event_store.is_undefined() {
			return Ok(());
		}

		for item in Array::from(&event_store).iter() {
			let event_arr = Array::from(&item);
			if event_arr.length() == 2 {
				let tree_idx =
					event_arr.get(0).as_f64().expect("bad event id") as u32;
				let event: Event = event_arr.get(1).unchecked_into();
				let event_type = format!("on{}", event.type_());
				if let Some(entity) = event_map
					.get(&(TreeIdx::new(tree_idx), &EventKey::new(&event_type)))
				{
					BeetEvent::trigger(
						&mut commands.entity(*entity),
						&event_type,
						event,
					);
				} else {
					bevybail!(
						"Event playback: could not find entity for event {}",
						tree_idx
					);
				}
			}
		}
		// we no longer need event store and event handler
		// because the event listeners have been hooked up
		js_sys::Reflect::delete_property(
			&global.unchecked_ref(),
			&constants.event_store.clone().into(),
		)
		.unwrap();
		js_sys::Reflect::delete_property(
			&global.unchecked_ref(),
			&constants.event_handler.clone().into(),
		)
		.unwrap();

		Ok(())
	})
}

mod beet_global_js {
	use wasm_bindgen::prelude::*;
	#[wasm_bindgen]
	extern "C" {
		#[wasm_bindgen]
		#[wasm_bindgen(thread_local_v2,js_name = globalThis)]
		pub static GLOBAL: JsValue;
	}
}

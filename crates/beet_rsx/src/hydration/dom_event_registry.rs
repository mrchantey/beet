use crate::prelude::*;
use js_sys::Array;
use js_sys::Reflect;
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::window;
use web_sys::Event;

pub struct EventRegistry;

thread_local! {
	static REGISTERED_EVENTS: RefCell<HashMap<(ElementIdx,String),Box<dyn Fn(JsValue)>>> = Default::default();
}

impl EventRegistry {
	fn trigger(key: &str, el_id: ElementIdx, value: JsValue) {
		REGISTERED_EVENTS.with(|current| {
			if let Some(func) = current.borrow().get(&(el_id, key.to_string()))
			{
				func(value);
			}
		});
	}

	fn register<T: 'static + JsCast>(
		key: &str,
		cx: &RsxContext,
		func: impl 'static + Fn(T),
	) {
		REGISTERED_EVENTS.with(|current| {
			current.borrow_mut().insert(
				(cx.element_idx(), key.to_string()),
				Box::new(move |e: JsValue| {
					func(e.unchecked_into());
				}),
			);
		});
	}
	pub fn register_onclick(
		key: &str,
		cx: &RsxContext,
		value: impl 'static + Fn(Event),
	) {
		Self::register(key, cx, value);
	}

	pub fn initialize() -> ParseResult<()> {
		let constants = CurrentHydrator::with(|h| h.html_constants().clone());
		hook_up_event_listeners(&constants)?;
		// TODO now the sweet loader is
		playback_prehydrate_events(&constants)?;
		Ok(())
	}
}


/// This may do nothing for one of several reasons:
/// - this hydration is happening before the page was mounted
/// - there was no pre-hydrated events script
fn playback_prehydrate_events(constants: &HtmlConstants) -> ParseResult<()> {
	sweet_loader_extern::GLOBAL.with(|global| {
		let event_store = Reflect::get(&global, &constants.event_store.into())
			.map_err(|_| {
				ParseError::Hydration("could not find event store".into())
			})?;
		if event_store.is_undefined() {
			return Ok(());
		}

		for item in Array::from(&event_store).iter() {
			let event_arr = Array::from(&item);
			if event_arr.length() == 2 {
				let id =
					event_arr.get(0).as_f64().expect("bad event id") as usize;
				let event: Event = event_arr.get(1).unchecked_into();
				let event_type = format!("on{}", event.type_());
				EventRegistry::trigger(&event_type, id, event.unchecked_into());
			}
		}
		// we no longer need event store and event handler
		// because the event listeners have been hooked up
		js_sys::Reflect::delete_property(
			&global.unchecked_ref(),
			&constants.event_store.into(),
		)
		.unwrap();
		js_sys::Reflect::delete_property(
			&global.unchecked_ref(),
			&constants.event_handler.into(),
		)
		.unwrap();

		Ok(())
	})
}

fn hook_up_event_listeners(constants: &HtmlConstants) -> ParseResult<()> {
	REGISTERED_EVENTS.with(|current| -> ParseResult<()> {
		let mut current = current.borrow_mut();
		let document = window().unwrap().document().unwrap();
		for ((el_id, key), func) in current.drain() {
			let query = format!("[{}='{}']", constants.id_key, el_id);

			let el =
				document.query_selector(&query).ok().flatten().ok_or_else(
					|| {
						ParseError::Hydration(format!(
							"could not find element with id: {}",
							query
						))
					},
				)?;
			el.remove_attribute(&key).unwrap();
			let closure = Closure::wrap(Box::new(move |e: JsValue| {
				func(e);
			}) as Box<dyn Fn(JsValue)>);
			el.add_event_listener_with_callback(
				&key.replace("on", ""),
				closure.as_ref().unchecked_ref(),
			)
			.unwrap();
			closure.forget();
		}
		Ok(())
	})
}

pub mod sweet_loader_extern {
	use wasm_bindgen::prelude::*;
	#[wasm_bindgen]
	extern "C" {
		#[wasm_bindgen]
		#[wasm_bindgen(thread_local_v2,js_name = globalThis)]
		pub static GLOBAL: JsValue;
	}
}

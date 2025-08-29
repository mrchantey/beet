use bevy::prelude::*;

#[cfg(target_arch = "wasm32")]
pub use send_wrapper::SendWrapper;

/// Uniform access to the value of an event's current target (e.g., <input value="...">)
pub trait EventExt {
	/// Shorthand for `event.current_target().value()`
	/// Panics if the event target does not have a 'value' property
	fn value(&self) -> String;
}

#[cfg(target_arch = "wasm32")]
pub struct DomEvent;

/// Lightweight mock event used on native (non-wasm) to simulate DOM events in tests.
pub struct MockEvent {
	pub value: String,
}
impl MockEvent {
	pub fn new(value: impl Into<String>) -> Self {
		Self {
			value: value.into(),
		}
	}
	pub fn key(&self) -> String { unimplemented!() }
	pub fn prevent_default(&self) { unimplemented!() }
}
impl EventExt for MockEvent {
	fn value(&self) -> String { self.value.clone() }
}

#[cfg(target_arch = "wasm32")]
impl EventExt for web_sys::Event {
	fn value(&self) -> String { wasm_utils::event_target_value(self) }
}

macro_rules! define_events {
	($(
		$event:literal | $struct:ident | $ty:path
	),* $(,)?) => {
		$(
			#[cfg(target_arch = "wasm32")]
			#[derive(Event,Deref)]
			pub struct $struct(pub SendWrapper<$ty>);
			#[cfg(not(target_arch = "wasm32"))]
			#[derive(Event,Deref)]
			pub struct $struct(pub MockEvent);
			#[cfg(target_arch = "wasm32")]
			impl EventExt for $struct{
				fn value(&self)->String{
					wasm_utils::event_target_value(self)
				}
			}

		)*

		#[cfg(target_arch = "wasm32")]
		impl DomEvent{
			pub fn trigger(
				commands: &mut bevy::ecs::system::EntityCommands,
				event_name: &str,
				ev: web_sys::Event,
			) {
				use wasm_bindgen::JsCast;
				match event_name.trim_start_matches("on") {
					$(
						$event => {
							let ev = ev.unchecked_into::<$ty>();
							commands.trigger($struct(SendWrapper::new(ev)));
						}
					)*
					_ => panic!("Unknown event: {event_name}"),
				}
			}
		}
	};
}
// list from https://www.w3schools.com/jsref/dom_obj_event.asp
define_events! {
"abort" | OnAbort | web_sys::UiEvent,
"afterprint" | OnAfterPrint | web_sys::Event,
"animationend" | OnAnimationEnd | web_sys::AnimationEvent,
"animationiteration" | OnAnimationIteration | web_sys::AnimationEvent,
"animationstart" | OnAnimationStart | web_sys::AnimationEvent,
"beforeprint" | OnBeforePrint | web_sys::Event,
"beforeunload" | OnBeforeUnload | web_sys::UiEvent,
"blur" | OnBlur | web_sys::FocusEvent,
"canplay" | OnCanPlay | web_sys::Event,
"canplaythrough" | OnCanPlayThrough | web_sys::Event,
"change" | OnChange | web_sys::Event,
"click" | OnClick | web_sys::MouseEvent,
"contextmenu" | OnContextMenu | web_sys::MouseEvent,
"copy" | OnCopy | web_sys::ClipboardEvent,
"cut" | OnCut | web_sys::ClipboardEvent,
"dblclick" | OnDblClick | web_sys::MouseEvent,
"drag" | OnDrag | web_sys::DragEvent,
"dragend" | OnDragEnd | web_sys::DragEvent,
"dragenter" | OnDragEnter | web_sys::DragEvent,
"dragleave" | OnDragLeave | web_sys::DragEvent,
"dragover" | OnDragOver | web_sys::DragEvent,
"dragstart" | OnDragStart | web_sys::DragEvent,
"drop" | OnDrop | web_sys::DragEvent,
"durationchange" | OnDurationChange | web_sys::Event,
"ended" | OnEnded | web_sys::Event,
"error" | OnError | web_sys::ProgressEvent,
"focus" | OnFocus | web_sys::FocusEvent,
"focusin" | OnFocusIn | web_sys::FocusEvent,
"focusout" | OnFocusOut | web_sys::FocusEvent,
"fullscreenchange" | OnFullscreenChange | web_sys::Event,
"fullscreenerror" | OnFullscreenError | web_sys::Event,
"hashchange" | OnHashChange | web_sys::HashChangeEvent,
"input" | OnInput | web_sys::InputEvent,
"invalid" | OnInvalid | web_sys::Event,
"keydown" | OnKeyDown | web_sys::KeyboardEvent,
"keypress" | OnKeyPress | web_sys::KeyboardEvent,
"keyup" | OnKeyUp | web_sys::KeyboardEvent,
"load" | OnLoad | web_sys::UiEvent,
"loadeddata" | OnLoadedData | web_sys::Event,
"loadedmetadata" | OnLoadedMetadata | web_sys::Event,
"loadstart" | OnLoadStart | web_sys::ProgressEvent,
"message" | OnMessage | web_sys::Event,
"mousedown" | OnMouseDown | web_sys::MouseEvent,
"mouseenter" | OnMouseEnter | web_sys::MouseEvent,
"mouseleave" | OnMouseLeave | web_sys::MouseEvent,
"mousemove" | OnMouseMove | web_sys::MouseEvent,
"mouseover" | OnMouseOver | web_sys::MouseEvent,
"mouseout" | OnMouseOut | web_sys::MouseEvent,
"mouseup" | OnMouseUp | web_sys::MouseEvent,
"mousewheel" | OnMouseWheel | web_sys::WheelEvent,
"offline" | OnOffline | web_sys::Event,
"online" | OnOnline | web_sys::Event,
"open" | OnOpen | web_sys::Event,
"pagehide" | OnPageHide | web_sys::PageTransitionEvent,
"pageshow" | OnPageShow | web_sys::PageTransitionEvent,
"paste" | OnPaste | web_sys::ClipboardEvent,
"pause" | OnPause | web_sys::Event,
"play" | OnPlay | web_sys::Event,
"playing" | OnPlaying | web_sys::Event,
"popstate" | OnPopState | web_sys::PopStateEvent,
"progress" | OnProgress | web_sys::Event,
"ratechange" | OnRateChange | web_sys::Event,
"resize" | OnResize | web_sys::UiEvent,
"reset" | OnReset | web_sys::Event,
"scroll" | OnScroll | web_sys::UiEvent,
"search" | OnSearch | web_sys::Event,
"seeked" | OnSeeked | web_sys::Event,
"seeking" | OnSeeking | web_sys::Event,
"select" | OnSelect | web_sys::UiEvent,
"show" | OnShow | web_sys::Event,
"stalled" | OnStalled | web_sys::Event,
"storage" | OnStorage | web_sys::StorageEvent,
"submit" | OnSubmit | web_sys::Event,
"suspend" | OnSuspend | web_sys::Event,
"timeupdate" | OnTimeUpdate | web_sys::Event,
"toggle" | OnToggle | web_sys::Event,
"touchcancel" | OnTouchCancel | web_sys::TouchEvent,
"touchend" | OnTouchEnd | web_sys::TouchEvent,
"touchmove" | OnTouchMove | web_sys::TouchEvent,
"touchstart" | OnTouchStart | web_sys::TouchEvent,
"transitionend" | OnTransitionEnd | web_sys::TransitionEvent,
"unload" | OnUnload | web_sys::UiEvent,
"volumechange" | OnVolumeChange | web_sys::Event,
"waiting" | OnWaiting | web_sys::Event,
"wheel" | OnWheel | web_sys::WheelEvent,
}

#[cfg(target_arch = "wasm32")]
mod wasm_utils {
	pub fn event_target_value(ev: &web_sys::Event) -> String {
		try_event_target_value(ev)
			.ok_or_else(|| format!("Failed to get value from event {ev:?}"))
			.unwrap()
	}

	fn try_event_target_value(ev: &web_sys::Event) -> Option<String> {
		let Some(target) = ev.current_target() else {
			return None;
		};
		use js_sys::Reflect;
		use wasm_bindgen::JsValue;

		let value = Reflect::get(&target, &JsValue::from_str("value")).ok()?;
		if value.is_undefined() || value.is_null() {
			return None;
		}
		value.as_string()
	}
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use crate::as_beet::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let (get, set) = signal(String::from("foo"));

		let mut app = App::new();
		app.add_plugins(ApplySnippetsPlugin);
		let world = app.world_mut();
		let ent = world
			.spawn(rsx! { <button onclick=move |ev| set(ev.value()) /> })
			.get::<Children>()
			.unwrap()[0];
		world.run_schedule(ApplySnippets);
		world
			.entity_mut(ent)
			// Thanks to From<MockEvent> for MouseEvent and Into<T> in BeetEvent::new,
			// this mirrors how downstream tests construct events.
			.trigger(OnClick(MockEvent::new("bar")));
		get().xpect().to_be("bar");
	}
}

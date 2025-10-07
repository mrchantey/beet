use beet_core::prelude::*;
#[cfg(target_arch = "wasm32")]
use send_wrapper::SendWrapper;
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

pub mod payloads {
	// First define the payload types for events
	macro_rules! define_payloads {
    ($($ty:ident),* $(,)?) => {
        $(
            #[cfg(target_arch = "wasm32")]
            pub type $ty = super::SendWrapper<web_sys::$ty>;
            #[cfg(not(target_arch = "wasm32"))]
            pub type $ty = super::MockEvent;
        )*
    }
	}
	define_payloads! {
		Event,
		UiEvent,
		AnimationEvent,
		FocusEvent,
		MouseEvent,
		ClipboardEvent,
		DragEvent,
		ProgressEvent,
		HashChangeEvent,
		InputEvent,
		KeyboardEvent,
		PageTransitionEvent,
		PopStateEvent,
		StorageEvent,
		TouchEvent,
		TransitionEvent,
		WheelEvent
	}
}
macro_rules! define_events {
	($(
		$event:literal | $struct:ident | $ty:path |$web_ty:path
	),* $(,)?) => {
		$(
			#[derive(EntityEvent)]
			pub struct $struct{
				entity: Entity,
				pub value: $ty
			}

			impl $struct{
				#[cfg(not(target_arch = "wasm32"))]
				pub fn new(value: $ty)-> impl 'static+Send+Sync+FnOnce(Entity)->Self{
				  move |entity| Self { entity, value }
				}
				#[cfg(target_arch = "wasm32")]
				pub fn new(value: $web_ty)-> impl 'static+Send+Sync+FnOnce(Entity)->Self{
					let value = SendWrapper::new(value);
				  move |entity| Self { entity, value }
				}
			}

			impl std::ops::Deref for $struct{
				type Target = $ty;
				fn deref(&self)->&Self::Target{
					&self.value
				}
			}

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
							let ev = ev.unchecked_into::<$web_ty>();
							commands.trigger($struct::new(ev));
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
"abort" | OnAbort | payloads::UiEvent | web_sys::UiEvent,
"afterprint" | OnAfterPrint | payloads::Event | web_sys::Event,
"animationend" | OnAnimationEnd | payloads::AnimationEvent | web_sys::AnimationEvent,
"animationiteration" | OnAnimationIteration | payloads::AnimationEvent | web_sys::AnimationEvent,
"animationstart" | OnAnimationStart | payloads::AnimationEvent | web_sys::AnimationEvent,
"beforeprint" | OnBeforePrint | payloads::Event | web_sys::Event,
"beforeunload" | OnBeforeUnload | payloads::UiEvent | web_sys::UiEvent,
"blur" | OnBlur | payloads::FocusEvent | web_sys::FocusEvent,
"canplay" | OnCanPlay | payloads::Event | web_sys::Event,
"canplaythrough" | OnCanPlayThrough | payloads::Event | web_sys::Event,
"change" | OnChange | payloads::Event | web_sys::Event,
"click" | OnClick | payloads::MouseEvent | web_sys::MouseEvent,
"contextmenu" | OnContextMenu | payloads::MouseEvent | web_sys::MouseEvent,
"copy" | OnCopy | payloads::ClipboardEvent | web_sys::ClipboardEvent,
"cut" | OnCut | payloads::ClipboardEvent | web_sys::ClipboardEvent,
"dblclick" | OnDblClick | payloads::MouseEvent | web_sys::MouseEvent,
"drag" | OnDrag | payloads::DragEvent | web_sys::DragEvent,
"dragend" | OnDragEnd | payloads::DragEvent | web_sys::DragEvent,
"dragenter" | OnDragEnter | payloads::DragEvent | web_sys::DragEvent,
"dragleave" | OnDragLeave | payloads::DragEvent | web_sys::DragEvent,
"dragover" | OnDragOver | payloads::DragEvent | web_sys::DragEvent,
"dragstart" | OnDragStart | payloads::DragEvent | web_sys::DragEvent,
"drop" | OnDrop | payloads::DragEvent | web_sys::DragEvent,
"durationchange" | OnDurationChange | payloads::Event | web_sys::Event,
"ended" | OnEnded | payloads::Event | web_sys::Event,
"error" | OnError | payloads::ProgressEvent | web_sys::ProgressEvent,
"focus" | OnFocus | payloads::FocusEvent | web_sys::FocusEvent,
"focusin" | OnFocusIn | payloads::FocusEvent | web_sys::FocusEvent,
"focusout" | OnFocusOut | payloads::FocusEvent | web_sys::FocusEvent,
"fullscreenchange" | OnFullscreenChange | payloads::Event | web_sys::Event,
"fullscreenerror" | OnFullscreenError | payloads::Event | web_sys::Event,
"hashchange" | OnHashChange | payloads::HashChangeEvent | web_sys::HashChangeEvent,
"input" | OnInput | payloads::InputEvent | web_sys::InputEvent,
"invalid" | OnInvalid | payloads::Event | web_sys::Event,
"keydown" | OnKeyDown | payloads::KeyboardEvent | web_sys::KeyboardEvent,
"keypress" | OnKeyPress | payloads::KeyboardEvent | web_sys::KeyboardEvent,
"keyup" | OnKeyUp | payloads::KeyboardEvent | web_sys::KeyboardEvent,
"load" | OnLoad | payloads::UiEvent | web_sys::UiEvent,
"loadeddata" | OnLoadedData | payloads::Event | web_sys::Event,
"loadedmetadata" | OnLoadedMetadata | payloads::Event | web_sys::Event,
"loadstart" | OnLoadStart | payloads::ProgressEvent | web_sys::ProgressEvent,
"message" | OnMessage | payloads::Event | web_sys::Event,
"mousedown" | OnMouseDown | payloads::MouseEvent | web_sys::MouseEvent,
"mouseenter" | OnMouseEnter | payloads::MouseEvent | web_sys::MouseEvent,
"mouseleave" | OnMouseLeave | payloads::MouseEvent | web_sys::MouseEvent,
"mousemove" | OnMouseMove | payloads::MouseEvent | web_sys::MouseEvent,
"mouseover" | OnMouseOver | payloads::MouseEvent | web_sys::MouseEvent,
"mouseout" | OnMouseOut | payloads::MouseEvent | web_sys::MouseEvent,
"mouseup" | OnMouseUp | payloads::MouseEvent | web_sys::MouseEvent,
"mousewheel" | OnMouseWheel | payloads::WheelEvent | web_sys::WheelEvent,
"offline" | OnOffline | payloads::Event | web_sys::Event,
"online" | OnOnline | payloads::Event | web_sys::Event,
"open" | OnOpen | payloads::Event | web_sys::Event,
"pagehide" | OnPageHide | payloads::PageTransitionEvent | web_sys::PageTransitionEvent,
"pageshow" | OnPageShow | payloads::PageTransitionEvent | web_sys::PageTransitionEvent,
"paste" | OnPaste | payloads::ClipboardEvent | web_sys::ClipboardEvent,
"pause" | OnPause | payloads::Event | web_sys::Event,
"play" | OnPlay | payloads::Event | web_sys::Event,
"playing" | OnPlaying | payloads::Event | web_sys::Event,
"popstate" | OnPopState | payloads::PopStateEvent | web_sys::PopStateEvent,
"progress" | OnProgress | payloads::Event | web_sys::Event,
"ratechange" | OnRateChange | payloads::Event | web_sys::Event,
"resize" | OnResize | payloads::UiEvent | web_sys::UiEvent,
"reset" | OnReset | payloads::Event | web_sys::Event,
"scroll" | OnScroll | payloads::UiEvent | web_sys::UiEvent,
"search" | OnSearch | payloads::Event | web_sys::Event,
"seeked" | OnSeeked | payloads::Event | web_sys::Event,
"seeking" | OnSeeking | payloads::Event | web_sys::Event,
"select" | OnSelect | payloads::UiEvent | web_sys::UiEvent,
"show" | OnShow | payloads::Event | web_sys::Event,
"stalled" | OnStalled | payloads::Event | web_sys::Event,
"storage" | OnStorage | payloads::StorageEvent | web_sys::StorageEvent,
"submit" | OnSubmit | payloads::Event | web_sys::Event,
"suspend" | OnSuspend | payloads::Event | web_sys::Event,
"timeupdate" | OnTimeUpdate | payloads::Event | web_sys::Event,
"toggle" | OnToggle | payloads::Event | web_sys::Event,
"touchcancel" | OnTouchCancel | payloads::TouchEvent | web_sys::TouchEvent,
"touchend" | OnTouchEnd | payloads::TouchEvent | web_sys::TouchEvent,
"touchmove" | OnTouchMove | payloads::TouchEvent | web_sys::TouchEvent,
"touchstart" | OnTouchStart | payloads::TouchEvent | web_sys::TouchEvent,
"transitionend" | OnTransitionEnd | payloads::TransitionEvent | web_sys::TransitionEvent,
"unload" | OnUnload | payloads::UiEvent | web_sys::UiEvent,
"volumechange" | OnVolumeChange | payloads::Event | web_sys::Event,
"waiting" | OnWaiting | payloads::Event | web_sys::Event,
"wheel" | OnWheel | payloads::WheelEvent | web_sys::WheelEvent,
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
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let (get, set) = signal(String::from("foo"));

		let mut world = World::new();
		let ent = world
			.spawn(rsx! { <button onclick=move |ev| set(ev.value())/> })
			.get::<Children>()
			.unwrap()[0];
		world
			.entity_mut(ent)
			// Thanks to From<MockEvent> for MouseEvent and Into<T> in BeetEvent::new,
			// this mirrors how downstream tests construct events.
			.trigger(OnClick::new(MockEvent::new("bar")));
		get().xpect_eq("bar");
	}
}

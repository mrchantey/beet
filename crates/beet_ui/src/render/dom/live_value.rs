//! [`LiveValue`]: what a served control holds, read off the DOM and written
//! into the world.
use beet_core::prelude::*;
use wasm_bindgen::JsCast;

/// What a control holds, read off the DOM: the text of an input, textarea or
/// select, or a checkbox's `checked`.
///
/// Two readers, one writer: [`read`](Self::read) is what an `input` or
/// `change` event carries, [`edited`](Self::edited) is what the served page
/// holds that the served markup did not say (a value the user changed before
/// the world existed), and [`write`](Self::write) lands either in the
/// entity's [`Value`], the direct write typing makes on the terminal.
pub(super) enum LiveValue {
	/// A checkbox's `checked`.
	Checked(bool),
	/// The text of an input, textarea or select.
	Text(String),
}

impl LiveValue {
	/// The control's live value.
	pub(super) fn read(target: &web_sys::EventTarget) -> Option<Self> {
		if let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() {
			match Self::toggles_on_click(input) {
				true => Self::Checked(input.checked()),
				false => Self::Text(input.value()),
			}
			.xsome()
		} else if let Some(area) =
			target.dyn_ref::<web_sys::HtmlTextAreaElement>()
		{
			Self::Text(area.value()).xsome()
		} else if let Some(select) =
			target.dyn_ref::<web_sys::HtmlSelectElement>()
		{
			Self::Text(select.value()).xsome()
		} else {
			None
		}
	}

	/// The control's live value only when it differs from what the served
	/// markup gave it: the value the user typed, ticked or picked before the
	/// world existed, which adoption hands to the world in place of a replay.
	pub(super) fn edited(target: &web_sys::Element) -> Option<Self> {
		if let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() {
			match Self::toggles_on_click(input) {
				true => (input.checked() != input.default_checked())
					.then(|| Self::Checked(input.checked())),
				false => (input.value() != input.default_value())
					.then(|| Self::Text(input.value())),
			}
		} else if let Some(area) =
			target.dyn_ref::<web_sys::HtmlTextAreaElement>()
		{
			(area.default_value().ok() != Some(area.value()))
				.then(|| Self::Text(area.value()))
		} else if let Some(select) =
			target.dyn_ref::<web_sys::HtmlSelectElement>()
		{
			let options = select.options();
			(0..options.length())
				.filter_map(|index| options.item(index))
				.filter_map(|option| {
					option.dyn_into::<web_sys::HtmlOptionElement>().ok()
				})
				.any(|option| option.selected() != option.default_selected())
				.then(|| Self::Text(select.value()))
		} else {
			None
		}
	}

	/// Whether an input's click is what changes its value (a checkbox, a
	/// radio), so its `checked` is its value and its activation is never
	/// replayed onto a value adoption already read.
	pub(super) fn toggles_on_click(input: &web_sys::HtmlInputElement) -> bool {
		matches!(input.type_().as_str(), "checkbox" | "radio")
	}

	/// Land in `entity`'s [`Value`]. Text re-parses into the value's own kind
	/// ([`Value::edit_text`]), so a number field mid-edit (`-`, `1e`) leaves
	/// the world's number and is flagged nothing, exactly as a rejected
	/// keystroke is on the terminal.
	pub(super) fn write(self, world: &mut World, entity: Entity) {
		let Some(mut value) = world.get_mut::<Value>(entity) else {
			return;
		};
		match self {
			Self::Checked(checked) => {
				value.set_if_neq(Value::Bool(checked));
			}
			Self::Text(text) => {
				if value
					.bypass_change_detection()
					.edit_text(|current| *current = text)
					.unwrap_or(false)
				{
					value.set_changed();
				}
			}
		}
	}
}

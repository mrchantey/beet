//! Error widgets styled by the material `Error`/`OnError` palette: [`Error`], a
//! box that wraps arbitrary children, and [`ErrorText`], a single colored span
//! for a message string.
use crate::prelude::*;
use beet_core::prelude::*;

/// A styled error box: paints its children on the material `Error`/`OnError`
/// palette (the `.error` rule). Wrap any failure content (a message, a broken
/// widget) in `<Error>...</Error>` to surface it inline.
#[template]
pub fn Error() -> impl Bundle {
	rsx! {
		<div {Classes::new([classes::ERROR])}><Slot/></div>
	}
}

/// Renders a colored error message in a `<span>` (the `.error-text` rule).
///
/// Reactive show/hide is the caller's responsibility: wrap the widget in an
/// effect or conditional render.
#[template]
pub fn ErrorText(#[prop(into)] message: String) -> impl Bundle {
	rsx! {
		<span {Classes::new([classes::ERROR_TEXT])}>{message}</span>
	}
}

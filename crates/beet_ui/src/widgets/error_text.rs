//! Error-text widget: a single-element widget styled by the `.error-text`
//! rule (`Error`/`OnError` tokens). The error message is supplied as a prop;
//! callers conditionally include the widget when an error is present.
use crate::prelude::*;
use beet_core::prelude::*;

/// Renders a colored error message in a `<span>`.
///
/// Reactive show/hide is the caller's responsibility — wrap the widget in an
/// effect or conditional render.
#[template]
pub fn ErrorText(#[prop(into)] message: String) -> impl Bundle {
	rsx! {
		<span {Classes::new([classes::ERROR_TEXT])}>{message}</span>
	}
}

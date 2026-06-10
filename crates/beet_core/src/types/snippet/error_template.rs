//! A build-subtree [`Template`] that always fails, surfacing a [`BevyError`]
//! through the substrate's [`TemplateError`] lifecycle rather than panicking.
//!
//! Its first use is `#[template(required)]`: required props are runtime-verified
//! input values, so a `#[template]` with an unset required prop builds an
//! [`ErrorTemplate`] carrying [`MissingProps`], which rides `TemplateError` and
//! surfaces through `LoadTemplate { is_error: true }`. The type stays generic
//! over the error so other build-time failures use the same path.
use crate::prelude::*;
use alloc::sync::Arc;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;
use core::panic::Location;

/// One or more `#[prop(required)]` props were not supplied to a `#[template]`.
///
/// A concrete error (not `bevyhow!`) so consumers can match on it; it flows
/// through the build channel via [`BevyError`]'s blanket `From`. The captured
/// [`Location`] is best effort: with `#[track_caller]` it usually resolves to
/// the `rsx!` call site.
#[derive(Debug, Clone, thiserror::Error)]
#[error("missing required props {props:?} (at {location})")]
pub struct MissingProps {
	/// Names of the unset required props.
	pub props: Vec<SmolStr>,
	/// Where the props were constructed.
	pub location: &'static Location<'static>,
}

/// A build-subtree [`Template`] that always fails to build with the wrapped
/// [`BevyError`].
///
/// Emitted by `#[template]` when a required prop is unset, but kept
/// error-agnostic for future build-time failures. The error rides
/// [`TemplateError`], never a panic.
#[derive(Debug, Clone)]
pub struct ErrorTemplate {
	// `Arc` because `BevyError` is not `Clone` and `clone_template` must hand
	// back a `Self`.
	error: Arc<BevyError>,
}

impl ErrorTemplate {
	/// Wrap the error this template will fail with.
	#[inline]
	pub fn new(error: impl Into<BevyError>) -> Self {
		Self {
			error: Arc::new(error.into()),
		}
	}
}

impl Template for ErrorTemplate {
	type Output = ();

	fn build_template(&self, _cx: &mut TemplateContext) -> Result<()> {
		// surface through the build channel (not a panic); `ErrorTemplate` is
		// itself an `Error`, so `BevyError`'s blanket `From<E: Error>` accepts it.
		Err(self.clone().into())
	}

	fn clone_template(&self) -> Self { self.clone() }
}

// `BevyError` is neither `Clone` nor `Error`, so a cloned `ErrorTemplate`
// re-surfaces its inner error by being an `Error` itself, delegating `Display`
// to the wrapped `BevyError`.
impl core::fmt::Display for ErrorTemplate {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		core::fmt::Display::fmt(&*self.error, f)
	}
}

impl core::error::Error for ErrorTemplate {}

//! A [`Scene`] that always fails to build, surfacing a [`BevyError`] through
//! the build channel rather than panicking.
//!
//! Its first use is `#[prop(required)]`: required-ness cannot be checked at
//! compile time (the rsx lowering builds props from `Props::default()
//! .with_x()`, which is well-typed regardless of which setters run), so a
//! `#[scene]` with required props validates at build time and returns an
//! [`ErrorScene`] carrying [`MissingProps`] when one is unset. See
//! `agent/plans/required_props.md` for the full rationale. The type is
//! intentionally generic over the error so other build-time failures can flow
//! through the same path.
use alloc::sync::Arc;
use beet_core::prelude::*;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;
use bevy::scene::ResolveContext;
use bevy::scene::ResolveSceneError;
use bevy::scene::ResolvedScene;
use bevy::scene::Scene;
use core::panic::Location;

/// One or more `#[prop(required)]` props were not supplied to a `#[scene]`.
///
/// A concrete error (not `bevyhow!`) so consumers can match on it; it flows
/// through the scene build channel via [`BevyError`]'s blanket `From`. The
/// captured [`Location`] is best effort: with `#[track_caller]` it usually
/// resolves to the rsx call site rather than the author's source.
#[derive(Debug, Clone, thiserror::Error)]
#[error("missing required props {props:?} (at {location})")]
pub struct MissingProps {
	/// Names of the unset required props.
	pub props: Vec<SmolStr>,
	/// Where the props were constructed.
	pub location: &'static Location<'static>,
}

/// A [`Scene`] that always fails to build with the wrapped [`BevyError`].
///
/// Emitted by the `#[scene]` macro when a required prop is unset, but kept
/// error-agnostic for future build-time failures. The error surfaces through
/// the build channel (bevy's error handler), never a panic — how to handle it
/// is the downstream user's choice.
#[derive(Debug, Clone)]
pub struct ErrorScene {
	// `Arc` because `BevyError` is not `Clone` and `clone_template` must hand
	// back a `Self`.
	error: Arc<BevyError>,
}

impl ErrorScene {
	/// Wrap the error this scene will fail with.
	#[inline]
	pub fn new(error: impl Into<BevyError>) -> Self {
		Self {
			error: Arc::new(error.into()),
		}
	}
}

impl Template for ErrorScene {
	type Output = ();

	fn build_template(
		&self,
		_ctx: &mut TemplateContext,
	) -> bevy::ecs::error::Result<Self::Output> {
		// surface through the build channel (not a panic); `ErrorScene` is itself
		// an `Error`, so `BevyError`'s blanket `From<E: Error>` accepts it
		Err(self.clone().into())
	}

	fn clone_template(&self) -> Self { self.clone() }
}

impl Scene for ErrorScene {
	fn resolve(
		self,
		_ctx: &mut ResolveContext,
		scene: &mut ResolvedScene,
	) -> Result<(), ResolveSceneError> {
		scene.push_bundle_template(self);
		Ok(())
	}
}

// `BevyError` is neither `Clone` nor `Error`, so a cloned `ErrorScene` re-surfaces
// its inner error by being an `Error` itself, delegating `Display` to the wrapped
// `BevyError`.
impl core::fmt::Display for ErrorScene {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		core::fmt::Display::fmt(&*self.error, f)
	}
}

impl core::error::Error for ErrorScene {}

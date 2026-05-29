//! Runtime support for `#[prop(required)]`: a [`Scene`] that always fails to
//! build, surfacing the missing prop names through the build channel rather
//! than panicking.
//!
//! Required-ness cannot be checked at compile time (the rsx lowering builds
//! props from `Props::default().with_x()`, which is well-typed regardless of
//! which setters run), so a `#[scene]` with required props validates at build
//! time: if a required prop is unset it returns an [`ErrorScene`] whose
//! [`Template`] build returns [`MissingProps`]. See
//! `agent/plans/required_props.md` for the full rationale.
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

/// A [`Scene`] that always fails to build with [`MissingProps`].
///
/// Emitted by the `#[scene]` macro when a required prop is unset. The error
/// surfaces through the build channel (bevy's error handler), never a panic —
/// how to handle it is the downstream user's choice.
#[derive(Debug, Clone)]
pub struct ErrorScene {
	missing: MissingProps,
}

impl ErrorScene {
	/// Wrap the [`MissingProps`] this scene will fail with.
	#[inline]
	pub fn new(missing: MissingProps) -> Self { Self { missing } }
}

impl Template for ErrorScene {
	type Output = ();

	fn build_template(
		&self,
		_ctx: &mut TemplateContext,
	) -> bevy::ecs::error::Result<Self::Output> {
		// surface through the build channel (not a panic)
		Err(self.missing.clone().into())
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

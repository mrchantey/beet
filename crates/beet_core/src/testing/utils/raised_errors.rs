//! Errors a test expects raised through a world's error handler.
use crate::prelude::*;
use bevy::ecs::error::BevyError;
use bevy::ecs::error::ErrorContext;
use bevy::ecs::error::FallbackErrorHandler;
use std::sync::Mutex;

/// Every error [`RaisedErrors`] recorded, with the test it was raised in.
static RAISED: Mutex<Vec<(Option<TestScope>, String)>> = Mutex::new(Vec::new());

/// The errors a test expects a world to raise through its error handler.
///
/// [`record`](Self::record) installs a handler keeping each error rather than
/// panicking on it, so the runner does not fail the test for an error it
/// asserts, and [`take`](Self::take) answers those the calling test raised.
/// The handler is a bare fn pointer with nowhere to capture, so the errors
/// land in one store, sorted by [`TestScope`].
///
/// ```
/// # use beet_core::prelude::*;
/// let mut world = World::new();
/// RaisedErrors::record(&mut world);
/// world.handle_command_error::<()>(bevyhow!("disk on fire"));
/// RaisedErrors::take().join("\n").xpect_contains("disk on fire");
/// ```
pub struct RaisedErrors;

impl RaisedErrors {
	/// Record every error `world` raises through its handler.
	pub fn record(world: &mut World) {
		world.insert_resource(FallbackErrorHandler(Self::handle));
	}

	/// The errors raised in the calling test since its last take, oldest
	/// first.
	pub fn take() -> Vec<String> {
		let scope = TestScope::current();
		let mut raised =
			RAISED.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
		let (taken, kept) = core::mem::take(&mut *raised)
			.into_iter()
			.partition::<Vec<_>, _>(|(raised_in, _)| *raised_in == scope);
		*raised = kept;
		taken.into_iter().map(|(_, text)| text).collect()
	}

	/// Keep `err`, attributed to the test it was raised in.
	fn handle(err: BevyError, _: ErrorContext) {
		RAISED
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner)
			.push((TestScope::current(), err.to_string()));
	}
}

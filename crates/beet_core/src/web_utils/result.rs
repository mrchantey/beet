//! Result conversion utilities for JavaScript interop.
//!
//! Provides extension methods for converting [`JsValue`] errors to
//! beet-compatible error types.

use crate::prelude::*;
use bevy::ecs::error::BevyError;
use extend::ext;
use wasm_bindgen::JsValue;

/// Extension trait for converting JS errors to [`BevyError`].
#[ext(name= BeetCoreJsResult)]
pub impl<T> Result<T, JsValue> {
	/// Maps a [`Result<T, JsValue>`] to a [`Result<T, BevyError>`].
	fn map_jserr(self) -> Result<T, BevyError> {
		self.map_err(|err| bevyhow!("{err:?}"))
	}
}

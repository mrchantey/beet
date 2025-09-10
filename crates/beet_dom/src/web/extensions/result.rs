use crate::bevyhow;
use bevy::ecs::error::BevyError;
use extend::ext;
use wasm_bindgen::JsValue;

#[ext]
pub impl<T> Result<T, JsValue> {
	/// Map a [`Result<T,JsValue>`] to a [`Result<T, BevyError>`].
	fn map_jserr(self) -> Result<T, BevyError> {
		self.map_err(|err| bevyhow!("{err:?}"))
	}
}

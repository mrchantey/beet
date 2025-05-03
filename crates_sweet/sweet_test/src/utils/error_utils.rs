use std::any::Any;


/// copied from rust
/// https://github.com/rust-lang/rust/blob/a25032cf444eeba7652ce5165a2be450430890ba/library/std/src/panic.rs#L125
pub fn payload_to_string(payload: &dyn Any) -> String {
	if let Some(s) = payload.downcast_ref::<&str>() {
		s.to_string()
	} else if let Some(s) = payload.downcast_ref::<String>() {
		s.clone()
	} else {
		"No panic payload".to_string()
	}
}

/// Attempt to downcast an any type into a string
pub fn try_downcast_str(value: &dyn std::any::Any) -> Option<String> {
	if let Some(str) = value.downcast_ref::<&str>() {
		Some(str.to_string())
	} else if let Some(str) = value.downcast_ref::<String>() {
		Some(str.clone())
	} else {
		None
	}
}

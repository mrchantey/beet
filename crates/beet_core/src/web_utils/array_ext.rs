//! Helper utilities for working with wasm array types.

/// Converts a [`js_sys::Array`] of JS strings into a Rust [`Vec<String>`].
///
/// # Panics
///
/// Panics if any element in the array is not a valid JS string.
pub fn into_vec_str(arr: js_sys::Array) -> Vec<String> {
	let len = arr.length();
	let mut out = Vec::<String>::with_capacity(len as usize);
	for i in 0..len {
		let item = arr.get(i).as_string().unwrap();
		out.push(item);
	}
	return out;
}

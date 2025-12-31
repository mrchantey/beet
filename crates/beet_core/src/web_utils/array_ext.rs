//! Helper utilities for working with wasm array types



pub fn into_vec_str(arr: js_sys::Array) -> Vec<String> {
	let len = arr.length();
	let mut out = Vec::<String>::with_capacity(len as usize);
	for i in 0..len {
		let item = arr.get(i).as_string().unwrap();
		out.push(item);
	}
	return out;
}

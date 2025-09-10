#[allow(unused)]
pub fn to_page(path: &str) {
	#[cfg(target_arch = "wasm32")]
	{
		web_sys::window()
			.unwrap()
			.location()
			.set_href(&path)
			.unwrap();
	}
	#[cfg(not(target_arch = "wasm32"))]
	unimplemented!();
}

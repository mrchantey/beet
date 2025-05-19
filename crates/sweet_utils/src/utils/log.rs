#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
	#[wasm_bindgen(js_namespace = console)]
	fn wasm_log(s: &str);
}

/// cross-platform way of logging a formatted value
#[macro_export]
macro_rules! log {
    ($($t:tt)*) => ({
        #[cfg(target_arch = "wasm32")]
		$crate::exports::web_sys::console::log_1(&(format!($($t)*).into()));
        #[cfg(not(target_arch = "wasm32"))]
        println!($($t)*);
    })
}
/// cross-platform way of error logging a formatted value
#[macro_export]
macro_rules! elog {
    ($($t:tt)*) => ({
        #[cfg(target_arch = "wasm32")]
		$crate::exports::web_sys::console::error_1(&(format!($($t)*).into()));
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!($($t)*);
    })
}




/// cross-platform way of logging a string
pub fn log_val(val: &str) {
	log!("{val}");
}
/// cross-platform way of logging a key and value
pub fn log_kvp(key: &str, val: impl std::fmt::Display) {
	log!("{key}: {val}");
}

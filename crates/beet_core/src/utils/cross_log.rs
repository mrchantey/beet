/// cross-platform way of logging a formatted value
#[macro_export]
macro_rules! cross_log {
    ($($t:tt)*) => ({
        #[cfg(target_arch = "wasm32")]
		$crate::exports::web_sys::console::log_1(&(format!($($t)*).into()));
        #[cfg(not(target_arch = "wasm32"))]
        println!($($t)*);
    })
}
/// cross-platform way of error logging a formatted value
#[macro_export]
macro_rules! cross_log_error {
    ($($t:tt)*) => ({
        #[cfg(target_arch = "wasm32")]
		$crate::exports::web_sys::console::error_1(&(format!($($t)*).into()));
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!($($t)*);
    })
}

/// cross-platform way of logging a formatted value,
/// using [`print!`] and [`stdout::flush`] for native logging
#[macro_export]
macro_rules! cross_log_noline {
    ($($t:tt)*) => ({
      #[cfg(target_arch = "wasm32")]
      	$crate::exports::web_sys::console::log_1(&($crate::_alloc::format!($($t)*).into()));
      #[cfg(not(target_arch = "wasm32"))]
        {
					#[cfg(feature = "std")]
					{
						print!($($t)*);
						use std::io::Write;
						std::io::stdout().flush().unwrap();
					}
					#[cfg(not(feature = "std"))]
					{
						tracing::info!($($t)*);
					}
				}
    })
}

/// cross-platform way of logging a formatted value
#[macro_export]
macro_rules! cross_log {
    ($($t:tt)*) => ({
      #[cfg(target_arch = "wasm32")]
      	$crate::exports::web_sys::console::log_1(&($crate::_alloc::format!($($t)*).into()));
      #[cfg(not(target_arch = "wasm32"))]
      {
				#[cfg(feature = "std")]
				println!($($t)*);
				#[cfg(not(feature = "std"))]
				tracing::info!($($t)*);
			}
    })
}
/// cross-platform way of error logging a formatted value
#[macro_export]
macro_rules! cross_log_error {
    ($($t:tt)*) => ({
      #[cfg(target_arch = "wasm32")]
        $crate::exports::web_sys::console::error_1(&($crate::_alloc::format!($($t)*).into()));
      #[cfg(not(target_arch = "wasm32"))]
      {
				#[cfg(feature = "std")]
				eprintln!($($t)*);
				#[cfg(not(feature = "std"))]
				tracing::error!($($t)*);
			}
    })
}

/// cross-platform way of logging a breakpoint with its span
#[macro_export]
macro_rules! breakpoint {
	() => {{
		$crate::cross_log!(
			"breakpoint at {}:{}:{}",
			file!(),
			line!(),
			column!()
		);
	}};
}

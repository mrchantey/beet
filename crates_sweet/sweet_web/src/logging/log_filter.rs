use super::*;
use js_sys::Reflect;
use wasm_bindgen::JsValue;

pub struct LogFilter {
	_log: ReplaceFunc,
}

impl LogFilter {
	pub fn new(name: &'static str, ignores: Vec<&'static str>) -> Self {
		let console =
			Reflect::get(&web_sys::window().unwrap(), &"console".into())
				.unwrap();

		let log_func = ReplaceFunc::func(console.clone(), name);
		let log =
			ReplaceFunc::new(console.clone(), name, move |val: JsValue| {
				println!("its a log! {:?} ", val);
				if val.is_string() {
					println!("its a string");
					let val = val.as_string().unwrap();
					if false == ignores.iter().any(|ignore| &val == ignore) {
						log_func.call1(&console, &val.into()).unwrap();
					}
				}
			});

		Self { _log: log }
	}

	pub fn default_log() -> Self {
		let ignores = vec![
			"[tower-livereload] connected...",
			"WebXR emulator extension overrides native WebXR API with polyfill."
		];
		LogFilter::new("log", ignores)
	}
	pub fn default_warn() -> Self {
		let ignores = vec![
	"Error with Permissions-Policy header: Origin trial controlled feature not enabled: 'browsing-topics'."
		];
		LogFilter::new("warn", ignores)
	}
}

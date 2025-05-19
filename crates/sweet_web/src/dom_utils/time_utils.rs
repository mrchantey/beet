use web_sys::window;

pub fn performance_now() -> f64 {
	window().unwrap().performance().unwrap().now()
}

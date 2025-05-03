use web_sys::window;


pub struct Location;

impl Location {
	pub fn navigate(path: &str) {
		window().unwrap().location().set_href(path).unwrap();
	}
	pub fn navigate_replace(path: &str) {
		window().unwrap().location().replace(path).unwrap();
	}
}

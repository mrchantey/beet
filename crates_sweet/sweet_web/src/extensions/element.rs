use extend::ext;
use wasm_bindgen::JsCast;
use web_sys::*;

#[ext]
pub impl HtmlElement {
	// fn get() -> Document { window().unwrap().document().unwrap()
	fn x_query_selector<T>(&self, selector: &str) -> Option<T>
	where
		T: JsCast,
	{
		self.query_selector(selector)
			.unwrap()
			.map(|el| el.dyn_into::<T>().unwrap())
	}
}

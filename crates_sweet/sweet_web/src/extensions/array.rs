use extend::ext;
use js_sys::Array;

#[ext]
pub impl Array {
	fn to_vec_str(&self) -> Vec<String> {
		self.iter()
			.map(|v| v.as_string().unwrap())
			.collect::<Vec<_>>()
	}
}

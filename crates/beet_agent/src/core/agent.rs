use std::borrow::Cow;

pub struct Context<'a> {
	content: Vec<Content<'a>>,
}




pub enum Content<'a> {
	Text(Cow<'a, str>),
	ImageBytes {
		bytes: Cow<'a, [u8]>,
		mime_type: Cow<'a, str>,
	},
}



pub trait ImageProvider {}


// #[cfg(test)]
// mod test {
// 	use crate::prelude::*;
// 	use sweet::prelude::*;

// 	#[test]
// 	fn works() {
// 		expect(true).to_be_false();

// 	}

// }

use beet_server::prelude::*;

#[rustfmt::skip]
fn main() {
	AppRouter::default()
		.run()
		.unwrap();
}

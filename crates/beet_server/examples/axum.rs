use beet_server::prelude::*;

#[rustfmt::skip]
fn main() {
	AppRouter::default()
		.serve()
		.unwrap();
}
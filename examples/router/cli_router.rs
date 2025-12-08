//! A basic example of using the beet router
use beet::prelude::*;

fn main() {
	let args = std::env::args().collect::<Vec<String>>();
	println!("CLI arguments: {:?}", args);
}

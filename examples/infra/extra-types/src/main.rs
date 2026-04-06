mod codegen {
	include!(concat!(env!("OUT_DIR"), "/types.rs"));
}


fn main() {
	println!("The best number is {}", codegen::BEST_NUMBER);
}

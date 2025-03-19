fn main() {
	#[cfg(feature = "setup")]
	beet_design::prelude::setup_config().export();
}

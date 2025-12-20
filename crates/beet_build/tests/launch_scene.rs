#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_core::prelude::*;

/// Tests the workflow of running a binary to output a `launch.ron` file.
#[test]
fn launch_scene() -> Result {
	let temp_dir = TempDir::new_workspace()?;
	let crate_dir =
		WsPathBuf::new("crates/beet_build/tests/pipeline_test_crate")
			.into_abs();
	fs_ext::copy_recursive(crate_dir, &temp_dir)?;

	// true.xpect_false();
	Ok(())
	// TempDir
}

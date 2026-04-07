use beet_core::prelude::*;


/// Initialize an opentofu directory, using the `./providers.tf.json`
pub async fn init(dir: &AbsPathBuf) -> Result {
	Process::new("tofu")
		.with_args(&["init"])
		.with_cwd(dir)
		.run_async()
		.await?;
	Ok(())
}

/// Validates the opentofu file, ie the `main.tf.json`
pub async fn validate(dir: &AbsPathBuf) -> Result<String> {
	Process::new("tofu")
		.with_args(&["validate", "-json"])
		.with_cwd(dir)
		.run_async_stdout()
		.await
}


/// Export the provider schema based on `./providers.tf.json`
pub async fn export_schema(dir: &AbsPathBuf) -> Result<String> {
	Process::new("tofu")
		.with_args(&["providers", "schema", "-json"])
		.with_cwd(dir)
		.run_async_stdout()
		.await
}

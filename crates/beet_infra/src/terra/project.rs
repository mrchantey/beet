use crate::prelude::terra::*;
use beet_core::prelude::*;





#[derive(Debug, Default, Clone, Get)]
pub struct Project {
	config: Config,
	dir: AbsPathBuf,
}
impl Project {
	pub fn new(dir: AbsPathBuf, config: Config) -> Self { Self { dir, config } }

	/// Write the config to a temporary directory and run `tofu validate`.
	///
	/// Returns the JSON output of `tofu validate -json` on success.
	#[cfg(not(target_arch = "wasm32"))]
	pub async fn validate(&self) -> Result<String> {
		self.write_config().await?;
		tofu::init(&self.dir).await?;
		tofu::validate(&self.dir).await
	}

	/// Write the configuration to a file.
	pub async fn write_config(&self) -> Result {
		let mut json = serde_json::to_vec_pretty(&self.config.to_json())?;
		json.push(b'\n');
		let path = self.dir.join("main.tf.json");
		fs_ext::write_async(path, &json).await?;
		Ok(())
	}
}

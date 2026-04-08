use crate::prelude::terra::*;
use beet_core::prelude::*;





#[derive(Debug, Default, Clone, Get)]
pub struct Project {
	config: Config,
	dir: AbsPathBuf,
}
impl Project {
	pub fn new(dir: AbsPathBuf, config: Config) -> Self { Self { dir, config } }
	/// Checks if a change is nessecary, if so writes the configuration
	/// and calls `tofu::init`
	pub async fn init(&self) -> Result {
		let bytes = serde_json::to_vec_pretty(&self.config.to_json())?;
		let path = self.dir.join("main.tf.json");
		if let Ok(current) = fs_ext::read_async(path.clone()).await
			&& current == bytes
		{
			// nothing to do
			return Ok(());
		}
		fs_ext::write_async(path, &bytes).await?;
		tofu::init(&self.dir).await?;
		Ok(())
	}

	/// Validates the opentofu file, ie the `main.tf.json`
	pub async fn validate(&self) -> Result<String> {
		self.init().await?;
		tofu::validate(&self.dir).await
	}

	/// Show execution plan
	pub async fn plan(&self) -> Result<String> {
		self.init().await?;
		tofu::plan(&self.dir).await
	}

	/// Apply the execution plan
	pub async fn apply(&self) -> Result<String> {
		self.init().await?;
		tofu::apply(&self.dir).await
	}

	/// Show the current state
	pub async fn show(&self) -> Result<String> {
		self.init().await?;
		tofu::show(&self.dir).await
	}

	/// List all resources in the state
	pub async fn list(&self) -> Result<String> {
		self.init().await?;
		tofu::list(&self.dir).await
	}

	/// Remove a resource from the state
	pub async fn remove(&self, resource: &str) -> Result<String> {
		self.init().await?;
		tofu::remove(&self.dir, resource).await
	}

	/// Destroy infrastructure
	pub async fn destroy(&self) -> Result<String> {
		self.init().await?;
		tofu::destroy(&self.dir).await
	}
}

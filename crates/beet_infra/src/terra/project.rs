use crate::prelude::terra::*;
use crate::prelude::*;
use beet_core::prelude::*;



#[derive(Debug, Clone, Get)]
pub struct Project {
	config: Config,
	dir: AbsPathBuf,
	backend: StackBackend,
}
impl Project {
	pub fn new(stack: &Stack, config: Config) -> Self {
		Self {
			dir: stack.work_directory().into_abs(),
			config,
			backend: stack.backend().clone(),
		}
	}

	/// Initialize the tofu project if required,
	/// checking if the config has changes and a lockfile exists
	async fn init(&self) -> Result {
		/// The lock file created by `tofu init` on successful completion.
		const LOCK_FILE: &str = ".terraform.lock.hcl";

		let bytes = serde_json::to_vec_pretty(&self.config.to_json())?;
		let path = self.dir.join("main.tf.json");
		let lock_path = self.dir.join(LOCK_FILE);
		let config_unchanged = fs_ext::read_async(path.clone())
			.await
			.is_ok_and(|current| current == bytes);
		let init_completed =
			fs_ext::exists_async(lock_path).await.unwrap_or(false);
		if config_unchanged && init_completed {
			trace!("tofu config unchanged, skipping init");
			return Ok(());
		}
		fs_ext::write_async(path, &bytes).await?;
		debug!("initializing tofu backend");
		tofu::ensure_backend_exists(&self.backend).await?;
		debug!("initializing tofu project");
		tofu::init(&self.dir).await?;
		Ok(())
	}

	/// Validates the OpenTofu config, ie the `main.tf.json`.
	/// Only downloads providers, no backend needed.
	pub async fn validate(&self) -> Result<String> {
		self.init().await?;
		tofu::validate(&self.dir).await
	}

	/// Show execution plan
	pub async fn plan(&self) -> Result<String> {
		self.init().await?;
		tofu::plan(&self.dir).await
	}

	/// Apply the execution plan.
	pub async fn apply(&self) -> Result<String> {
		self.init().await?;
		tofu::apply(&self.dir).await
	}

	/// Show the current state.
	pub async fn show(&self) -> Result<String> {
		self.init().await?;
		tofu::show(&self.dir).await
	}

	/// List all resources in the state.
	pub async fn list(&self) -> Result<String> {
		self.init().await?;
		tofu::list(&self.dir).await
	}

	/// Remove a resource from the state.
	pub async fn remove(&self, resource: &str) -> Result<String> {
		self.init().await?;
		tofu::remove(&self.dir, resource).await
	}

	/// Destroy infrastructure.
	pub async fn destroy(&self) -> Result<String> {
		self.init().await?;
		tofu::destroy(&self.dir).await
	}
}

use crate::prelude::terra::*;
use crate::prelude::*;
use beet_core::prelude::*;



#[derive(Debug, Clone, Deref, Get)]
pub struct Project {
	config: Config,
	#[deref]
	stack: Stack,
}
impl Project {
	pub fn new(stack: &Stack, config: Config) -> Self {
		Self {
			config,
			stack: stack.clone(),
		}
	}

	/// The absolute working directory for the tofu project.
	fn dir(&self) -> AbsPathBuf { self.work_directory().into_abs() }

	/// Initialize the tofu project if required,
	/// checking if the config has changes, a lockfile exists,
	/// and the backend type matches the current config.
	async fn init(&self) -> Result {
		/// The lock file created by `tofu init` on successful completion.
		const LOCK_FILE: &str = ".terraform.lock.hcl";

		let dir = self.dir();
		let bytes = serde_json::to_vec_pretty(&self.config.to_json())?;
		let config_path = dir.join("main.tf.json");
		let lock_path = dir.join(LOCK_FILE);
		let config_unchanged = fs_ext::read_async(config_path.clone())
			.await
			.is_ok_and(|current| current == bytes);
		let init_completed =
			fs_ext::exists_async(lock_path).await.unwrap_or(false);
		if config_unchanged && init_completed {
			trace!("tofu config unchanged, skipping init");
			return Ok(());
		}
		fs_ext::write_async(config_path, &bytes).await?;
		debug!("initializing tofu backend");
		self.backend().ensure_exists().await?;
		debug!("initializing tofu project");
		tofu::init(&dir, *self.reconfigure()).await?;
		Ok(())
	}

	/// Validates the OpenTofu config, ie the `main.tf.json`.
	/// Only downloads providers, no backend needed.
	pub async fn validate(&self) -> Result<String> {
		self.init().await?;
		tofu::validate(&self.dir()).await
	}

	/// Show execution plan
	pub async fn plan(&self) -> Result<String> {
		self.init().await?;
		tofu::plan(&self.dir()).await
	}

	/// Apply the execution plan.
	pub async fn apply(&self) -> Result<String> {
		self.init().await?;
		tofu::apply(&self.dir()).await
	}

	/// Show the current state.
	pub async fn show(&self) -> Result<String> {
		self.init().await?;
		tofu::show(&self.dir()).await
	}

	/// List all resources in the state.
	pub async fn list(&self) -> Result<String> {
		self.init().await?;
		tofu::list(&self.dir()).await
	}

	/// Remove a resource from the state.
	pub async fn remove(&self, resource: &str) -> Result<String> {
		self.init().await?;
		tofu::remove(&self.dir(), resource).await
	}

	/// Destroy infrastructure.
	/// - runs tofu destroy, tearing down all infrastructure
	/// - removes the state file from the state bucket
	/// - removes the working directory
	pub async fn destroy(&self) -> Result {
		self.init().await?;
		tofu::destroy(&self.dir()).await?;
		self.backend()
			.provider()
			.remove(&self.backend_path())
			.await?;
		fs_ext::remove_async(&self.dir()).await?;
		Ok(())
	}
	/// Destroys infrastructure without initializing, moving forward
	/// with each step, even if other parts fail ie dir exists but no backend state.
	/// - clears stale state locks from interrupted runs
	/// - runs tofu destroy (lock-free), tearing down all infrastructure
	/// - removes the state file from the state bucket
	/// - removes the working directory
	pub async fn force_destroy(&self) {
		self.backend().clear_stale_locks();
		tofu::destroy_force(&self.dir()).await.ok();
		self.backend()
			.provider()
			.remove(&self.backend_path())
			.await
			.ok();
		fs_ext::remove_async(&self.dir()).await.ok();
	}
}

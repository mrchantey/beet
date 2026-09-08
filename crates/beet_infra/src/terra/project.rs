use crate::prelude::terra::*;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::Blob;
use beet_net::prelude::BlobStoreProvider;

#[derive(Debug, Clone, Deref, Get)]
pub struct Project {
	config: Config,
	#[deref]
	stack: ResolvedStack,
	/// This launch's mechanics: the state backend the project drives and the
	/// work directory it drives it in.
	deployment: Deployment,
	/// The stack's artifacts client, built once at construction so teardown
	/// does not rebuild it.
	artifacts: ArtifactsClient,
	/// The variables the stack's blocks declared, so a verb that renders can
	/// resolve the ones that are resource CONTENT before invoking tofu.
	variables: Vec<crate::types::Variable>,
}
impl Project {
	pub fn new(
		stack: ResolvedStack,
		deployment: Deployment,
		config: Config,
	) -> Self {
		Self::new_with_variables(stack, deployment, config, Vec::new())
	}

	/// A project that also knows the variables its blocks declared, which is
	/// what lets `plan` and `apply` be truthful about content values rather than
	/// falling through to a default.
	pub fn new_with_variables(
		stack: ResolvedStack,
		deployment: Deployment,
		config: Config,
		variables: Vec<crate::types::Variable>,
	) -> Self {
		let artifacts = deployment.artifacts_client(&stack);
		Self {
			config,
			stack,
			deployment,
			artifacts,
			variables,
		}
	}

	/// Resolve every declared variable that is resource CONTENT, ie one whose
	/// value decides what a resource is rather than how it is configured at
	/// runtime.
	///
	/// Ambient variables are deliberately left alone: they are supplied by a
	/// deploy pipeline through `apply_with_vars`, and a `plan` that invented
	/// them would be lying in the other direction. Content variables carry no
	/// terraform default, so an unresolved one refuses rather than renders.
	async fn content_vars(&self) -> Result<Vec<(SmolStr, SmolStr)>> {
		let mut resolved = Vec::new();
		for variable in self.variables.iter() {
			let Some(value) = self.resolve_content(variable).await? else {
				continue;
			};
			resolved.push((variable.key().clone(), value));
		}
		Ok(resolved)
	}

	/// Read one content variable's value, or `None` if it is ambient and so
	/// falls through to its declared default.
	///
	/// An absent parameter is a hard error: a content variable has no default,
	/// so there is nothing to fall back to, and inventing one is how a `p=`
	/// revocation gets published.
	async fn resolve_content(
		&self,
		variable: &crate::types::Variable,
	) -> Result<Option<SmolStr>> {
		if let Some(value) = variable.fixed_value() {
			return Ok(Some(value.clone()));
		}
		let Some(parameter) = variable.ssm_parameter() else {
			return Ok(None);
		};
		match read_parameter(self.stack.region(), parameter).await? {
			Some(value) => Ok(Some(SmolStr::new(value))),
			None => bevybail!(
				"no value at parameter store `{parameter}`, which variable \
				`{}` is the content of. It is minted by the stack's `deploy` \
				verb; run that rather than a bare apply, which would publish an \
				empty value.",
				variable.key()
			),
		}
	}

	/// [`required_vars`](Self::required_vars) plus the resolved content values,
	/// the `-var` set any rendering invocation needs.
	async fn render_vars(&self) -> Result<Vec<(SmolStr, SmolStr)>> {
		let mut vars = self.required_vars()?;
		vars.extend(self.content_vars().await?);
		Ok(vars)
	}

	/// The `-var` set for a TEARDOWN: every content variable gets a value, but
	/// an unresolvable one falls back to empty rather than refusing.
	///
	/// A content variable carries no default precisely so a render cannot invent
	/// one, and terraform therefore demands a value for it even here. But the
	/// reason to refuse does not apply to a destroy: nothing is being published,
	/// the resource is going away, and a stack must always be tearable down. A
	/// teardown blocked because a parameter it is about to orphan had already
	/// been deleted would be a trap, not a guard.
	async fn destroy_vars(&self) -> Vec<(SmolStr, SmolStr)> {
		let mut vars = self.required_vars().unwrap_or_default();
		for variable in self.variables.iter().filter(|var| var.is_content()) {
			let value = self
				.resolve_content(variable)
				.await
				.ok()
				.flatten()
				.unwrap_or_default();
			vars.push((variable.key().clone(), value));
		}
		vars
	}

	/// The absolute working directory for the tofu project, where its rendered
	/// config, its lockfile and the deploy's scratch files live.
	pub fn work_dir(&self) -> AbsPathBuf {
		self.deployment.work_directory(&self.stack).into_abs()
	}

	/// Short alias for [`Self::work_dir`], the tofu driver's `-chdir`.
	fn dir(&self) -> AbsPathBuf { self.work_dir() }

	/// The blob holding this project's tofu state.
	pub fn state_file(&self) -> Blob { self.deployment.state_file(&self.stack) }

	/// The state backend this project's state lives in.
	fn backend(&self) -> &StackBackend { self.deployment.backend() }

	/// The key this project's state is written under.
	fn backend_path(&self) -> SmolPath {
		self.deployment.backend_path(&self.stack)
	}

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
		tofu::init(&dir).await?;
		Ok(())
	}

	/// `-var` pairs every state-touching invocation needs, currently just the
	/// [`StateEncryption`] passphrase (empty when the deploy has it off),
	/// resolved fresh from its environment variable each call.
	fn required_vars(&self) -> Result<Vec<(SmolStr, SmolStr)>> {
		self.deployment.state_encryption().vars()
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
		tofu::plan(&self.dir(), &self.render_vars().await?).await
	}

	/// Apply the execution plan.
	pub async fn apply(&self) -> Result<String> {
		self.init().await?;
		tofu::apply(&self.dir(), &self.render_vars().await?).await
	}

	/// Apply the execution plan with Terraform variables, narrowed to `targets`
	/// (resource addresses, ie a layer's [`Config::layer_targets`]) when
	/// non-empty. `vars` is merged with [`Self::required_vars`], so a caller
	/// never needs to thread the state encryption passphrase through itself.
	pub async fn apply_with_vars(
		&self,
		vars: &[(SmolStr, SmolStr)],
		targets: &[String],
	) -> Result<String> {
		self.init().await?;
		let mut all_vars = self.render_vars().await?;
		all_vars.extend_from_slice(vars);
		tofu::apply_with_vars(&self.dir(), &all_vars, targets).await
	}

	/// Show the current state.
	pub async fn show(&self) -> Result<String> {
		self.init().await?;
		tofu::show(&self.dir(), &self.required_vars()?).await
	}

	/// Read a specific output value from the tofu state.
	pub async fn output(&self, name: &str) -> Result<String> {
		self.init().await?;
		tofu::output(&self.dir(), &self.required_vars()?, name).await
	}

	/// List all resources in the state.
	pub async fn list(&self) -> Result<String> {
		self.init().await?;
		tofu::list(&self.dir(), &self.required_vars()?).await
	}

	/// Remove a resource from the state.
	pub async fn remove(&self, resource: &str) -> Result<String> {
		self.init().await?;
		tofu::remove(&self.dir(), &self.required_vars()?, resource).await
	}

	/// Destroy infrastructure.
	/// - runs tofu destroy, tearing down all infrastructure
	/// - removes the state file from the state bucket
	/// - removes the working directory
	pub async fn destroy(&self) -> Result {
		self.init().await?;
		tofu::destroy(&self.dir(), &self.destroy_vars().await).await?;
		self.destroy_common().await;
		Ok(())
	}
	/// Destroys infrastructure moving forward
	/// with each step, even if other parts fail ie dir exists but no backend state.
	/// - clears stale state locks from interrupted runs
	/// - runs tofu destroy (lock-free), tearing down all infrastructure
	/// - removes the state file from the state bucket
	/// - removes the working directory
	pub async fn force_destroy(&self) {
		// init so destroy can access providers and state even after partial cleanup
		self.init().await.ok();
		self.backend().clear_stale_locks();
		let vars = self.destroy_vars().await;
		tofu::destroy_force(&self.dir(), &vars).await.ok();
		self.destroy_common().await;
	}

	async fn destroy_common(&self) {
		// remove state file
		self.backend()
			.provider()
			.remove(&self.backend_path())
			.await
			.ok();
		// remove S3 native lock file left by interrupted runs
		let lock_path =
			SmolPath::new(format!("{}.tflock", self.backend_path()));
		self.backend().provider().remove(&lock_path).await.ok();
		self.artifacts.store().store_remove().await.ok();
		fs_ext::remove_async(&self.dir()).await.ok();
	}
}

/// Read a parameter store value, the one deploy-side capability a
/// [`Project`] reaches for beyond the tofu CLI.
///
/// Gated because parameter store is an `actions`-side binding and `actions` is
/// the deploy feature. A build without it links no aws cli, and also runs no
/// `plan`/`apply`, so the unresolvable case is unreachable rather than
/// degraded: it says so rather than pretending the parameter was empty, which
/// is the failure mode this whole path exists to prevent.
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
async fn read_parameter(region: &str, name: &str) -> Result<Option<String>> {
	crate::prelude::ssm_ext::get(region, name).await
}

#[cfg(not(all(feature = "deploy", not(target_arch = "wasm32"))))]
async fn read_parameter(_region: &str, name: &str) -> Result<Option<String>> {
	bevybail!(
		"cannot read parameter store `{name}`: this binary was built without the \
		`deploy` feature, so it has no aws cli to read it with"
	)
}

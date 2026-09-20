use crate::prelude::terra::*;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::Blob;

#[derive(Debug, Clone, Deref, Get)]
pub struct Project {
	config: Config,
	#[deref]
	stack: ResolvedStack,
	/// This launch's mechanics: the state backend the project drives and the
	/// work directory it drives it in.
	deployment: Deployment,
	/// The variables the stack's blocks declared, so a verb that renders can
	/// resolve the ones that are resource CONTENT before invoking tofu.
	variables: Vec<crate::types::Variable>,
	/// The stack's secret store, which the content variables read through;
	/// absent on a project built without one, where a content variable is an
	/// error at render.
	#[cfg(feature = "vault")]
	secrets: Option<crate::types::SecretStore>,
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
		Self {
			config,
			stack,
			deployment,
			variables,
			#[cfg(feature = "vault")]
			secrets: None,
		}
	}

	/// The project with the secret store its content variables read through.
	#[cfg(feature = "vault")]
	pub fn with_secret_store(
		mut self,
		secrets: crate::types::SecretStore,
	) -> Self {
		self.secrets = Some(secrets);
		self
	}

	/// The stack's secret store, an error naming the resolution when the
	/// project was built without one.
	#[cfg(feature = "vault")]
	pub fn secret_store(&self) -> Result<&crate::types::SecretStore> {
		self.secrets.as_ref().ok_or_else(|| {
			bevyhow!(
				"project `{}--{}` was built without a secret store: resolve it \
				through `Project::resolve` (which reads the stack's declaration) \
				rather than `RenderScope::project`",
				self.stack.app_name(),
				self.stack.stage()
			)
		})
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
	/// An absent secret is a hard error: a content variable has no default,
	/// so there is nothing to fall back to, and inventing one is how a `p=`
	/// revocation gets published.
	async fn resolve_content(
		&self,
		variable: &crate::types::Variable,
	) -> Result<Option<SmolStr>> {
		if let Some(value) = variable.fixed_value() {
			return Ok(Some(value.clone()));
		}
		let Some(secret) = variable.secret_ref() else {
			return Ok(None);
		};
		match self.read_secret(secret).await? {
			Some(value) => Ok(Some(SmolStr::new(value))),
			// the resource this variable is the content of does not exist yet,
			// and the block reading it emits nothing for empty
			None if variable.absent_is_empty() => {
				info!(
					"no value at secret `{}` yet, so variable `{}` resolves \
					empty and publishes nothing",
					secret.label(),
					variable.key()
				);
				Ok(Some(SmolStr::default()))
			}
			None => bevybail!(
				"no value at secret `{}`, which variable `{}` is the content \
				of. It is minted by the stack's `deploy` verb; run that rather \
				than a bare apply, which would publish an empty value.",
				secret.label(),
				variable.key()
			),
		}
	}

	/// Read a secret through the stack's store, the one deploy-side
	/// capability a [`Project`] reaches for beyond the tofu CLI.
	///
	/// A build without the `vault` feature links no provider, and also runs
	/// no `plan`/`apply` that would need one, so the unresolvable case says so
	/// rather than pretending the secret was empty, which is the failure mode
	/// this whole path exists to prevent.
	async fn read_secret(&self, secret: &SecretRef) -> Result<Option<String>> {
		cfg_if! {
			if #[cfg(feature = "vault")] {
				self.secret_store()?.get(secret).await
			} else {
				bevybail!(
					"cannot read secret `{}`: this binary was built without the \
					`vault` feature, so it has no secret store to read it from",
					secret.label()
				)
			}
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
	pub fn work_dir(&self) -> AbsPath {
		self.deployment.work_directory(&self.stack).into_abs()
	}

	/// Short alias for [`Self::work_dir`], the tofu driver's `-chdir`.
	fn dir(&self) -> AbsPath { self.work_dir() }

	/// The blob holding this project's tofu state.
	pub fn state_file(&self) -> Result<Blob> {
		self.deployment.state_file(&self.stack)
	}

	/// The state backend this project's state lives in.
	fn backend(&self) -> &StackBackend { self.deployment.backend() }

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
		// init evaluates the encryption config, so it needs the passphrase
		tofu::init(&dir, &self.required_vars()?).await?;
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

	/// Apply with `resources` (addresses) replaced, see
	/// [`tofu::apply_replacing`]: the rotation of every secret an apply
	/// derives.
	pub async fn apply_replacing(
		&self,
		resources: &[String],
	) -> Result<String> {
		self.init().await?;
		tofu::apply_replacing(
			&self.dir(),
			&self.render_vars().await?,
			resources,
		)
		.await
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

	/// Run `tofu destroy`, and nothing else.
	///
	/// What the old `destroy` swept afterwards, the state object, the native S3
	/// lock and the work dir, belongs to `StackTeardown` now. Those converge
	/// BEFORE the apply, so under the one rule that teardown order is
	/// convergence order reversed they tear down after it, which is exactly
	/// where a destroy group puts them.
	///
	/// `force` is the recovery path for a state nothing holds but a lock left by
	/// an interrupted run: it clears the stale lock and destroys lock-free, and
	/// it destroys even from a project too partially cleaned up to `init`.
	pub async fn tofu_destroy(&self, force: bool) -> Result<String> {
		if force {
			// a half-cleaned project may not init; there is still state to destroy
			self.init().await.ok();
			self.backend().clear_stale_locks();
			return tofu::destroy_force(
				&self.dir(),
				&self.destroy_vars().await,
			)
			.await;
		}
		self.init().await?;
		tofu::destroy(&self.dir(), &self.destroy_vars().await).await
	}
}

#[cfg(all(test, feature = "vault"))]
mod test {
	use super::*;
	use crate::types::Variable;
	use crate::types::test_support::*;

	/// A content variable resolves through the stack's store before any
	/// render: a present secret is its value, an absent optional one is
	/// empty, an absent required one refuses, and a project built without
	/// a store says so rather than inventing a value.
	#[beet_core::test]
	async fn content_variables_read_the_secret_store() {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let config = deployment.create_config(&stack);
		let variables = vec![
			Variable::secret("dkim", SecretRef::new("dkim-example-com")),
			Variable::secret_optional("tlsa", SecretRef::new("mail-tlsa")),
			Variable::param("ambient"),
		];
		let store = memory_secret_store(&stack);
		store
			.create(
				&SecretRef::new("dkim-example-com"),
				"MIIB",
				None,
				SecretRotation::Remint,
			)
			.await
			.unwrap();
		let project = Project::new_with_variables(
			stack.clone(),
			deployment.clone(),
			config.clone(),
			variables.clone(),
		);
		project
			.content_vars()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("without a secret store");
		let project = project.with_secret_store(store.clone());
		project.content_vars().await.unwrap().xpect_eq(vec![
			("dkim".into(), "MIIB".into()),
			("tlsa".into(), SmolStr::default()),
		]);
		// a required secret that is absent refuses, naming the deploy verb
		let project = Project::new_with_variables(
			stack.clone(),
			deployment,
			config,
			vec![Variable::secret("other", SecretRef::new("absent"))],
		)
		.with_secret_store(store);
		project
			.content_vars()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("secret `absent`")
			.xpect_contains("`deploy` verb");
		// a teardown falls back to empty rather than refusing
		project
			.destroy_vars()
			.await
			.contains(&("other".into(), SmolStr::default()))
			.xpect_true();
	}
}

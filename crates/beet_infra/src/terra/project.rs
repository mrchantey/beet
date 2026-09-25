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

	/// Re-encrypt this stack's state under the passphrase its declared
	/// variable holds NOW, reading it under the one in `retiring`
	/// (`<variable>_OLD` unless given): the stack half of a rotation, run once
	/// per stack after `secrets/set <retiring> --copy=<variable>` kept the old
	/// value and `secrets/set <variable> --generate` minted the new.
	///
	/// OpenTofu keys pbkdf2's salt by key-provider name, so two passphrases
	/// cannot both be `main` and a swap crosses the plaintext bridge instead:
	/// one rewrite reading under the retiring value and writing plaintext
	/// ([`StateBridge::Decrypt`]), one reading plaintext and writing under
	/// the current ([`StateBridge::Encrypt`], the `state_migrate` shape). A
	/// state that was never encrypted takes the second alone, and one
	/// already under the current value is left as it is, so the verb re-runs
	/// safely and the same command encrypts a plaintext stack.
	pub async fn rotate_state(&self, retiring: Option<&str>) -> Result<String> {
		let name = format!("{}--{}", self.stack.app_name(), self.stack.stage());
		let current = self.deployment.state_encryption().clone();
		let StateEncryption::Passphrase { env_var, .. } = &current else {
			bevybail!("`{name}` has state encryption off: nothing to rotate");
		};
		let retiring = retiring
			.map(SmolStr::new)
			.unwrap_or_else(|| format!("{env_var}_OLD").into());
		let encrypt = self.with_state_encryption(
			current.clone().with_bridge(StateBridge::Encrypt),
		);
		match self.state_is_encrypted().await? {
			None => bevybail!("`{name}` has no state yet: nothing to rotate"),
			Some(false) => {
				let serial = encrypt.rewrite_state().await?;
				self.init().await?;
				format!(
					"encrypted the plaintext state of `{name}` under \
					`{env_var}` (serial {serial})"
				)
				.xok()
			}
			Some(true) => {
				info!(
					"probing whether `{name}` already reads under `{env_var}`"
				);
				if self.reads_state().await {
					return format!(
						"the state of `{name}` is already encrypted under \
						`{env_var}`"
					)
					.xok();
				}
				if env_ext::var(retiring.as_str()).is_err() {
					bevybail!(
						"`{name}` is encrypted under a passphrase other than \
						`{env_var}` and `{retiring}` is not set. Keep the \
						retiring value beside the new one BEFORE minting it: \
						`secrets/set {retiring} --copy={env_var} \
						--role=env_var`, then `secrets/set {env_var} \
						--generate ..`, and `secrets/rm {retiring}` once every \
						stack has rotated"
					);
				}
				self.with_state_encryption(
					current
						.clone()
						.with_env_var(retiring.clone())
						.with_bridge(StateBridge::Decrypt),
				)
				.rewrite_state()
				.await?;
				let serial = encrypt.rewrite_state().await?;
				self.init().await?;
				format!(
					"rotated the state of `{name}` from `{retiring}` to \
					`{env_var}` (serial {serial})"
				)
				.xok()
			}
		}
	}

	/// This project reading and writing its state under `encryption` instead:
	/// the same stack, backend, work dir and rendered resources, which is how
	/// a rotation pushes the state across a [`StateBridge`].
	fn with_state_encryption(&self, encryption: StateEncryption) -> Self {
		Self {
			config: self.config.clone().with_state_encryption(&encryption),
			deployment: self
				.deployment
				.clone()
				.with_state_encryption(encryption),
			..self.clone()
		}
	}

	/// Whether the backend holds this stack's state encrypted, plaintext, or
	/// (`None`) not at all, read off the object itself: an encrypted state
	/// is an envelope carrying `encrypted_data`, a plaintext one the state.
	async fn state_is_encrypted(&self) -> Result<Option<bool>> {
		let blob = self.state_file()?;
		if !blob.exists().await? {
			return None.xok();
		}
		serde_json::from_slice::<serde_json::Value>(&blob.get().await?)?
			.get("encrypted_data")
			.is_some()
			.xmap(Some)
			.xok()
	}

	/// Whether the state reads under this project's own encryption: the init
	/// reads it, and so does the pull when the init was skipped as current.
	async fn reads_state(&self) -> bool {
		let Ok(vars) = self.required_vars() else {
			return false;
		};
		self.init().await.is_ok()
			&& tofu::state_pull(&self.dir(), &vars).await.is_ok()
	}

	/// Pull the state under this project's encryption and push it straight
	/// back with its serial bumped, so the backend rewrites it under the
	/// PRIMARY method: one crossing of a [`StateBridge`]. The bump is what
	/// makes tofu write at all, since an unchanged state is not persisted.
	async fn rewrite_state(&self) -> Result<u64> {
		self.init().await?;
		let vars = self.required_vars()?;
		let dir = self.dir();
		let mut state = tofu::state_pull(&dir, &vars)
			.await?
			.xmap(|json| serde_json::from_str::<serde_json::Value>(&json))?;
		let serial = state["serial"]
			.as_u64()
			.ok_or_else(|| bevyhow!("the pulled state carries no serial"))?
			+ 1;
		state["serial"] = serial.into();
		// the state holds what an apply derives, so it never sits readable
		let file = dir.join("rewrite.tfstate");
		fs_ext::write_private(&file, serde_json::to_vec(&state)?)?;
		let pushed = tofu::state_push(&dir, &vars, &file).await;
		fs_ext::remove_async(&file).await.ok();
		pushed?;
		serial.xok()
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

/// Native only: drives the real `tofu` against a local backend.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod rotation {
	use super::*;

	/// A rotation reads the state under the retiring passphrase and rewrites
	/// it under the current one across the plaintext bridge; a plaintext
	/// state is encrypted by the same call, a state already under the
	/// current value is left alone, and a missing retiring variable names the
	/// `--copy` step. Eight tofu invocations, so well past the default budget.
	#[beet_core::test(timeout_ms = 120_000)]
	async fn rotates_the_state_passphrase() {
		const OLD: &str = "BEET_TEST_ROTATE_OLD";
		const NEW: &str = "BEET_TEST_ROTATE_NEW";
		// SAFETY: test-only names no other test reads; pbkdf2 wants 16+ chars
		unsafe {
			env_ext::set_var(OLD, "the-retiring-passphrase").ok();
			env_ext::set_var(NEW, "the-current-passphrase").ok();
		}
		let (stack, deployment, dir) = ResolvedStack::default_local();
		// a backend of this test's own: the shared default would carry this
		// state, encrypted, into every other test's init
		let deployment = deployment
			.with_backend(LocalBackend::new(dir.path().join("state")));
		let under = |encryption: StateEncryption| {
			let deployment =
				deployment.clone().with_state_encryption(encryption);
			let config = deployment.create_config(&stack);
			Project::new(stack.clone(), deployment, config)
		};
		// a state written before encryption, as a stack predating it holds
		let plaintext = under(StateEncryption::None);
		plaintext.init().await.unwrap();
		let seed = plaintext.dir().join("seed.tfstate");
		fs_ext::write_private(
			&seed,
			br#"{"version":4,"terraform_version":"1.6.0","serial":1,"lineage":"0b8ad4af-4aed-60c5-89f2-6c4ad06ccc07","outputs":{},"resources":[]}"#,
		)
		.unwrap();
		tofu::state_push(&plaintext.dir(), &[], &seed)
			.await
			.unwrap();
		plaintext
			.state_is_encrypted()
			.await
			.unwrap()
			.xpect_eq(Some(false));

		let old = under(StateEncryption::passphrase(OLD));
		old.rotate_state(None)
			.await
			.unwrap()
			.xpect_contains("encrypted the plaintext state");
		old.state_is_encrypted().await.unwrap().xpect_eq(Some(true));
		old.reads_state().await.xpect_true();

		let new = under(StateEncryption::passphrase(NEW));
		new.rotate_state(Some(OLD))
			.await
			.unwrap()
			.xpect_contains(format!("from `{OLD}` to `{NEW}`"));
		new.reads_state().await.xpect_true();
		old.reads_state().await.xpect_false();
		new.rotate_state(Some(OLD))
			.await
			.unwrap()
			.xpect_contains("already encrypted");
		old.rotate_state(Some("BEET_TEST_ROTATE_MISSING"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("--copy=BEET_TEST_ROTATE_OLD");
	}
}

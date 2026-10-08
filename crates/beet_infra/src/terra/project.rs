use crate::prelude::terra::*;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::Blob;

#[derive(Debug, Clone, Deref, Get)]
pub struct Project {
	config: Config,
	#[deref]
	stack: ResolvedStack,
	/// Where this project's state lives, resolved: a project cannot be built
	/// without one, which is what makes every render and every state address
	/// below total. See [`StackBackend::resolve`].
	backend: ResolvedBackend,
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
	/// Written in the work directory once `tofu init` has finished, holding the
	/// sha256 of the config it initialized. The rendered `main.tf.json` cannot
	/// say that itself: it is on disk *before* init runs, so a run that wrote a
	/// new config and then failed (an unreachable state bucket, an interrupted
	/// init) would otherwise leave the config looking initialized, and the next
	/// verb would drive tofu against the backend the last finished init
	/// configured.
	const INIT_STAMP: &'static str = ".beet-tofu-init";

	pub fn new(
		stack: ResolvedStack,
		deployment: Deployment,
		config: Config,
		backend: ResolvedBackend,
	) -> Self {
		Self::new_with_variables(stack, deployment, config, backend, Vec::new())
	}

	/// A project that also knows the variables its blocks declared, which is
	/// what lets `plan` and `apply` be truthful about content values rather than
	/// falling through to a default.
	pub fn new_with_variables(
		stack: ResolvedStack,
		deployment: Deployment,
		config: Config,
		backend: ResolvedBackend,
		variables: Vec<crate::types::Variable>,
	) -> Self {
		// the backend block is the one part of the config that needs the
		// resolution, so it is rendered where the two meet rather than at
		// `Deployment::create_config`, which knows only the declaration
		let mut config = config;
		config.set_backend(
			backend.to_json(&deployment.backend_path(&stack).to_string()),
		);
		Self {
			config,
			backend,
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
		self.backend
			.store()?
			.blob(self.deployment.backend_path(&self.stack))
			.xok()
	}

	/// Initialize the tofu project unless the last init finished against this
	/// exact config, see [`Self::INIT_STAMP`].
	async fn init(&self) -> Result {
		/// The directory `tofu init` creates, holding the backend it configured
		/// and every provider it installed: deleting it un-initializes the
		/// project, so the stamp alone is not enough.
		const TOFU_DIR: &str = ".terraform";

		let dir = self.dir();
		let bytes = serde_json::to_vec_pretty(&self.config.to_json())?;
		let digest = digest_ext::hex::<sha2::Sha256>(&bytes);
		let config_path = dir.join("main.tf.json");
		let stamp_path = dir.join(Self::INIT_STAMP);
		let config_initialized =
			fs_ext::read_to_string_async(stamp_path.clone())
				.await
				.is_ok_and(|stamped| stamped == digest);
		let init_completed = fs_ext::exists_async(dir.join(TOFU_DIR))
			.await
			.unwrap_or(false);
		if config_initialized && init_completed {
			trace!("tofu config unchanged since its init, skipping init");
			return Ok(());
		}
		fs_ext::write_async(config_path, &bytes).await?;
		debug!("initializing tofu backend");
		self.backend().ensure_exists().await?;
		debug!("initializing tofu project");
		// init evaluates the encryption config, so it needs the passphrase
		tofu::init(&dir, &self.required_vars()?).await?;
		// only now, see `INIT_STAMP`
		fs_ext::write_async(stamp_path, digest).await?;
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

	/// The writes [`apply_with_vars`](Self::apply_with_vars) would make with
	/// the same `vars` and `targets`, planned against the state alone, see
	/// [`tofu::planned_changes`].
	pub async fn planned_changes(
		&self,
		vars: &[(SmolStr, SmolStr)],
		targets: &[String],
	) -> Result<Vec<tofu::PlannedChange>> {
		self.init().await?;
		let mut all_vars = self.render_vars().await?;
		all_vars.extend_from_slice(vars);
		tofu::planned_changes(&self.dir(), &all_vars, targets).await
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
	/// Two writes to the backend, both encrypted under the current
	/// passphrase: the state is pulled under the retiring one and pushed back
	/// under the current one ([`rewrite_state`](Self::rewrite_state)), so the
	/// only readable copy is a private temp file between the two calls.
	///
	/// **A rotation must never write a state the passphrase it retires can
	/// read, and must never write a plaintext one.** The backend is versioned,
	/// so such a write is not overwritten, it is kept.
	///
	/// A state that was never encrypted takes [`StateCrossing::FromPlaintext`]
	/// instead, so the same command encrypts a plaintext stack. One already
	/// under the current value is left as it is, so the verb re-runs safely.
	///
	/// **Interrupted between the two writes**, the state is readable under the
	/// current passphrase but salted under the secondary key name, which a
	/// steady-state read does not address: every verb of that stack fails.
	/// Re-running this finishes it, and does so WITHOUT the retiring value,
	/// which matters because the operator is told to remove that once the
	/// fleet is done and the straggler is exactly the stack that crashed.
	pub async fn rotate_state(&self, retiring: Option<&str>) -> Result<String> {
		let name = format!("{}--{}", self.stack.app_name(), self.stack.stage());
		let current = self.deployment.state_encryption().clone();
		let StateEncryption::Passphrase { env_var, .. } = &current else {
			bevybail!("`{name}` has state encryption off: nothing to rotate");
		};
		let retiring = retiring
			.map(SmolStr::new)
			.unwrap_or_else(|| format!("{env_var}_OLD").into());
		match self.state_is_encrypted().await? {
			None => bevybail!("`{name}` has no state yet: nothing to rotate"),
			Some(false) => {
				// one project: `unencrypted` needs no key material, so it can
				// be a fallback and the read and the write share a config
				let serial = self
					.with_state_encryption(
						current.clone().crossing(StateCrossing::FromPlaintext),
					)
					.rewrite_state()
					.await?;
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
				// a rotation interrupted between its two writes leaves the
				// state readable under the CURRENT passphrase but salted under
				// the secondary key name, which a steady-state read does not
				// address. Re-running recovers it, and the retiring value is
				// not needed to, so check that before demanding it.
				if self
					.with_state_encryption(
						current.clone().crossing(StateCrossing::FromSecondary),
					)
					.reads_state()
					.await
				{
					let serial = self
						.with_state_encryption(
							current
								.clone()
								.crossing(StateCrossing::FromSecondary),
						)
						.rewrite_state()
						.await?;
					self.init().await?;
					return format!(
						"finished an interrupted rotation of `{name}`: its \
						state was parked mid-crossing and now reads under \
						`{env_var}` (serial {serial})"
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
				// out of the primary key holding the retiring passphrase, then
				// back into it holding the current one: two writes, both
				// encrypted, because the salt of the state being read is
				// stored under the key provider's NAME
				self.with_state_encryption(
					current.clone().crossing(StateCrossing::FromRetiring(
						retiring.clone(),
					)),
				)
				.rewrite_state()
				.await?;
				let serial = self
					.with_state_encryption(
						current.clone().crossing(StateCrossing::FromSecondary),
					)
					.rewrite_state()
					.await?;
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
	/// a rotation makes each half of a [`StateCrossing`].
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

	/// Pull the state through this project's [`StateCrossing`] and push it
	/// straight back with its serial bumped, so the backend rewrites it under
	/// the crossing's WRITE method. The bump is what makes tofu write at all,
	/// since an unchanged state is not persisted.
	///
	/// **Every crossing writes encrypted**, so the only readable copy is one
	/// private temp file between the pull and the push. That is the property
	/// the whole rotation design exists to keep: the backend is versioned, so
	/// a plaintext write would be retained rather than overwritten, and the
	/// encryption a rotation is renewing would be defeated by the rotation.
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
		let backend = LocalBackend::default().into();
		let project = Project::new_with_variables(
			stack.clone(),
			deployment.clone(),
			config.clone(),
			backend,
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
			LocalBackend::default().into(),
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
mod init {
	use super::*;

	/// An init that did not finish does not look finished. The rendered config
	/// reaches disk before init runs, so a run that re-pointed the backend and
	/// then failed leaves exactly this behind: the new `main.tf.json` beside the
	/// previous init. The next verb must init again, because tofu refuses every
	/// state-touching command while its recorded backend differs from the config.
	#[beet_core::test(timeout_ms = 120_000)]
	async fn reinits_after_an_unfinished_init() {
		let (stack, deployment, dir) = ResolvedStack::default_local();
		let under = |state_dir: &str| {
			let local = LocalBackend::new(dir.path().join(state_dir));
			let deployment = deployment.clone().with_backend(local.clone());
			let config = deployment.create_config(&stack);
			Project::new(stack.clone(), deployment, config, local.into())
		};
		let first = under("state-a");
		first.init().await.unwrap();
		let second = under("state-b");
		fs_ext::write_async(
			second.dir().join("main.tf.json"),
			serde_json::to_vec_pretty(&second.config.to_json()).unwrap(),
		)
		.await
		.unwrap();
		// inits, so this reads `state-b`; a skipped init makes tofu refuse
		second.show().await.unwrap();
	}
}

/// Native only: drives the real `tofu` against a local backend.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod rotation {
	use super::*;

	/// A rotation reads the state under the retiring passphrase and rewrites
	/// it under the current one; a plaintext state is encrypted by the same
	/// call, a state already under the current value is left alone, and a
	/// missing retiring variable names the `--copy` step.
	///
	/// The assertion that matters most is the last one: **no write a rotation
	/// makes may be readable**. A backend is versioned, so a plaintext or
	/// stale-key write is kept rather than overwritten, and a rotation that
	/// made one would publish the very state it was run to protect. The
	/// check watches every version of the local backend's file, since the
	/// finished object says nothing about what it replaced.
	///
	/// Well past the default budget: a dozen tofu invocations.
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
		let local = LocalBackend::new(dir.path().join("state"));
		let deployment = deployment.with_backend(local.clone());
		let under = |encryption: StateEncryption| {
			let deployment =
				deployment.clone().with_state_encryption(encryption);
			let config = deployment.create_config(&stack);
			Project::new(
				stack.clone(),
				deployment,
				config,
				local.clone().into(),
			)
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

		// every write the rotation makes, watched as it makes them: the
		// finished object says nothing about what it replaced
		let state_dir = dir.path().join("state");
		let writes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
		let watched = writes.clone();
		let done =
			std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
		let stop = done.clone();
		let watching = std::thread::spawn(move || {
			let mut last: Vec<Vec<u8>> = Vec::new();
			while !stop.load(std::sync::atomic::Ordering::Relaxed) {
				let Ok(entries) = std::fs::read_dir(&state_dir) else {
					continue;
				};
				for entry in entries.flatten() {
					let Ok(bytes) = std::fs::read(entry.path()) else {
						continue;
					};
					// a tofu state, settled or in flight; anything else in the
					// directory (a lock, a backup) is not what this watches
					if serde_json::from_slice::<serde_json::Value>(&bytes)
						.is_ok_and(|json| {
							json.get("lineage").is_some()
								|| json.get("encrypted_data").is_some()
						}) && !last.contains(&bytes)
					{
						last.push(bytes.clone());
						watched.lock().unwrap().push(bytes);
					}
				}
				std::thread::sleep(Duration::from_millis(2));
			}
		});

		let new = under(StateEncryption::passphrase(NEW));
		new.rotate_state(Some(OLD))
			.await
			.unwrap()
			.xpect_contains(format!("from `{OLD}` to `{NEW}`"));
		new.reads_state().await.xpect_true();
		// the retiring passphrase reads nothing the rotation left behind,
		// which a crossing through plaintext would have failed
		done.store(true, std::sync::atomic::Ordering::Relaxed);
		watching.join().unwrap();
		let seen = writes.lock().unwrap().clone();
		seen.is_empty().xpect_false();
		for bytes in &seen {
			serde_json::from_slice::<serde_json::Value>(bytes)
				.unwrap()
				.get("encrypted_data")
				.xpect_some();
		}
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

		// a rotation interrupted between its two writes: the state is readable
		// under the current passphrase but salted under the secondary name, so
		// every verb of the stack fails. Re-running finishes it, and WITHOUT
		// the retiring value, which by then may be gone.
		under(
			StateEncryption::passphrase(NEW)
				.crossing(StateCrossing::FromRetiring(NEW.into())),
		)
		.rewrite_state()
		.await
		.unwrap();
		new.reads_state().await.xpect_false();
		new.rotate_state(Some("BEET_TEST_ROTATE_MISSING"))
			.await
			.unwrap()
			.xpect_contains("interrupted rotation");
		new.reads_state().await.xpect_true();
	}
}

//! `<AwsDeployer/>`: this repo's own AWS deployer user, its policies and the
//! pair its credential document holds.

use crate::actions::aws_cli_ext;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;

/// `<AwsDeployer/>` — the repo's AWS deploy credential, declared at the
/// entry's root like `<Secrets/>`: one IAM user for the credential document,
/// one managed policy per app the entry declares ([`DeployerPolicy`], lowered
/// from what the stacks render), one runtime boundary per app and stage
/// ([`RuntimeBoundary`]), and one access key, sealed into the document as
/// `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` with the policies it was
/// converged against in the key id's record.
///
/// Nobody mints it by hand: a deploy checks the sealed grants against the
/// declarations before it writes anything and refuses naming `--elevated`
/// when they fall short, and the elevated deploy ([`DeployGate`]) converges
/// it here, as an administrator session bought with the code from the phone.
///
/// ## Why derived outside tofu rather than declared in a stack
///
/// Everything else about a stack is declared, and a deployer user could be:
/// `aws_iam_user` and `aws_iam_access_key` are bound, so the apply that
/// provisions a stack could provision the identity applying it. It should not.
/// A deployer declared in its own stack has to hold `iam:PutUserPolicy` on its
/// own user to plan, which is an administrator with extra steps; the stack's
/// `destroy` would delete the credential it is authenticating with; and the
/// tier cannot close, since the apply that mints the first deployer has to run
/// as some earlier one. So the identity lives OUTSIDE tofu, and what the block
/// would have bought (a policy derived from the declarations rather than
/// written by hand) is bought here instead, from the same render.
///
/// ## Which credential an elevated deploy runs this as
///
/// Minting an IAM user is an administrator's act, so the converge does not run
/// as the deployer it converges: its scoped policy grants no `iam:` action over
/// itself. It runs as an administrator session ([`AdminSession::acquire`]),
/// the live one `beet admin` left on tmpfs when it has time to spare, else an
/// hour bought with the code from the phone, kept nowhere.
///
/// ## Every stage that deploys
///
/// A launch renders the stacks of one stage, and a credential is per repo, so
/// the policies are lowered at the launch's stage AND at `prod`
/// ([`DeployCredential::render_stages`]): an elevated `dev` deploy can never
/// narrow what prod needs.
///
/// ## The order of operations, which this enforces
///
/// Per app it converges TWO kinds of managed policy: `<app>--<stage>--runtime-boundary`
/// ([`RuntimeBoundary`], the ceiling on what a deploy may create) and
/// `<app>--deploy` ([`DeployerPolicy`], what the deployer may do). The
/// boundary goes first, and the deploy policy carries the conditions that name
/// it only once every principal the render names already WEARS it, which this
/// asks the account rather than taking on trust.
///
/// That is not caution, it is the only order that works: a `StringNotEquals`
/// deny on `iam:PermissionsBoundary` is true for a principal carrying no
/// boundary, so conditioning the policy before the apply that attaches the
/// first boundaries would deny that apply and block every deploy of the app
/// until an administrator intervened. So the first converge of an app with
/// live uncapped principals writes an unconditioned policy and says so; the
/// apply caps them; the next elevated deploy closes the door. An app whose
/// principals are not at the account yet is conditioned from birth.
///
/// ## What converges
///
/// - the user, `<repo>-deployer`, created when absent. Named for the
///   CREDENTIAL DOCUMENT rather than for an app, since a document holds one
///   `AWS_ACCESS_KEY_ID` however many apps a repo deploys, and derived from
///   the workspace directory because a repo has no other name.
/// - one managed policy per app, attached to the user, rewritten only when its
///   document changed (managed rather than inline: a user's inline budget is
///   2048 characters for ALL of its policies together, which three apps of a
///   repo exceed). A policy of an app this launch does not declare is left
///   alone, so a repo whose entries declare different apps converges each from
///   its own entry, and the key id's record keeps every entry's grants.
/// - an access key, minted when the document holds none of this user's, or on
///   `--roll`. The pair is sealed before the key it replaces is deleted, and
///   the new one is proven with `sts:GetCallerIdentity` before either. A
///   current pair's records are resealed, values unchanged, when their note,
///   roll or grants are not what this writes now.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct AwsDeployer;

impl DeployCredentialProvider for AwsDeployer {
	fn id(&self) -> &'static str { "aws" }

	fn human_factor(&self) -> &'static str {
		"the six-digit code from the phone, for an administrator session"
	}

	fn records(&self) -> &'static [&'static str] {
		&[Self::KEY_ID, Self::KEY_SECRET]
	}

	fn guards(&self, declared: &str) -> bool {
		Self::REMOVAL_DENIED.contains(&declared)
			|| Self::WRITE_DENIED.contains(&declared)
	}

	fn protects(
		&self,
		change: &tofu::PlannedChange,
		stack: &ResolvedStack,
	) -> Option<String> {
		if stack.stage() == BootstrapConfig::DEFAULT_STAGE {
			return None;
		}
		let kind = change.resource_type.as_str();
		let removes = matches!(change.action.as_str(), "delete" | "replace");
		let denied = (removes && Self::REMOVAL_DENIED.contains(&kind))
			|| Self::WRITE_DENIED.contains(&kind);
		denied.then(|| {
			format!(
				"this deploy would {} `{}`, which no stored deployer may \
				outside the `{}` stage: it is a change a deployer is denied \
				without the code from the phone, since it can make data final",
				change.action,
				change.address,
				BootstrapConfig::DEFAULT_STAGE
			)
		})
	}

	fn status(
		&self,
		caller: AsyncEntity,
	) -> SendBoxedFuture<Result<CredentialStatus>> {
		let this = self.clone();
		Box::pin(async move { this.status_of(&caller).await })
	}

	fn elevate(
		&self,
		caller: AsyncEntity,
		ask: ElevationAsk,
	) -> SendBoxedFuture<Result<CredentialElevation>> {
		let this = self.clone();
		Box::pin(async move { this.elevate_with(&caller, ask).await })
	}

	fn describe(&self, caller: AsyncEntity) -> SendBoxedFuture<Result<String>> {
		Box::pin(async move { Self::describe_of(&caller).await })
	}
}

/// One app's deploy policy, the boundary of each stage this launch rendered,
/// and the principals those stacks name, which is what decides whether the
/// deploy policy may carry the boundary conditions.
struct AppPolicies {
	deploy: DeployerPolicy,
	/// Per STAGE, since a boundary's content is one stage's names. The deploy
	/// policy is per app, and its condition matches any of them.
	boundaries: BTreeMap<SmolStr, RuntimeBoundary>,
	/// `(kind, name)` per principal, ie `("role", "beet-site--prod--..")`.
	principals: Vec<(&'static str, SmolStr)>,
}

impl AwsDeployer {
	/// The record names the pair is sealed under, which are the sdk's own
	/// variables: what the document holds is what the environment gets.
	const KEY_ID: &'static str = "AWS_ACCESS_KEY_ID";
	const KEY_SECRET: &'static str = "AWS_SECRET_ACCESS_KEY";

	/// The group a first converge seals into; a record already sealed stays
	/// in its own.
	const DEFAULT_GROUP: &'static str = SecretsDocument::DEFAULT_GROUP;

	/// The types whose removal [`DeployerPolicy`] denies a deployer without
	/// the code from the phone outside the default stage (its
	/// `IrreversibleNeedsMfa` and `DataVolumeNeedsMfa` statements), so a plain
	/// deploy refuses a plan that removes one rather than meeting the deny
	/// mid-apply.
	const REMOVAL_DENIED: &'static [&'static str] = &[
		"aws_s3_bucket",
		"aws_dynamodb_table",
		"aws_db_instance",
		"aws_ebs_volume",
	];

	/// The types ANY write to is denied the same way: a bucket's versioning
	/// and its lifecycle, which together are what makes its history
	/// recoverable, so suspending one or shortening the other is a deletion.
	/// Creating a bucket outside `dev` writes both, so it is an elevated
	/// deploy, once.
	const WRITE_DENIED: &'static [&'static str] = &[
		"aws_s3_bucket_versioning",
		"aws_s3_bucket_lifecycle_configuration",
	];

	/// Where the sealed pair stands against the policies this launch's
	/// declarations render, by the grants its key id's record carries. The
	/// record holds every entry's apps, so only this launch's are compared.
	async fn status_of(
		&self,
		caller: &AsyncEntity,
	) -> Result<CredentialStatus> {
		let Some(apps) = Self::apps(caller).await? else {
			return CredentialStatus::not_needed().xok();
		};
		let grants = Self::grant_lines(&apps)?;
		let held = DeployCredential::sealed(caller, Self::KEY_ID).await;
		let ours = |line: &str| Self::belongs(&apps, line);
		CredentialStatus::compare(
			self.id(),
			Self::KEY_ID,
			&grants,
			held.as_ref(),
			Some(&ours),
		)
		.xok()
	}

	/// Whether a grant `line` names a policy of one of `apps`.
	fn belongs(apps: &[AppPolicies], line: &str) -> bool {
		apps.iter()
			.any(|app| line.starts_with(&format!("{}--", app.deploy.app())))
	}

	/// One line per policy this launch's apps render: its name and a digest of
	/// its document, the deploy policy unconditioned since whether it carries
	/// the boundary conditions is an account read, not a declaration.
	fn grant_lines(apps: &[AppPolicies]) -> Result<Vec<String>> {
		let digest = |document: &Value| {
			digest_ext::hex::<sha2::Sha256>(document.to_string().as_bytes())
				[..16]
				.to_string()
		};
		let mut lines = Vec::new();
		for app in apps {
			lines.push(format!(
				"{} {}",
				app.deploy.policy_name(),
				digest(&app.deploy.to_json())
			));
			for boundary in
				app.boundaries.values().filter(|held| !held.is_empty())
			{
				lines.push(format!(
					"{} {}",
					boundary.policy_name(),
					digest(&boundary.to_json())
				));
			}
		}
		lines.sort();
		lines.xok()
	}

	/// Buy an administrator session, converge the user, every boundary and
	/// policy, and the key, and when `ask.run`, hand the deploy the session.
	async fn elevate_with(
		&self,
		caller: &AsyncEntity,
		ask: ElevationAsk,
	) -> Result<CredentialElevation> {
		let Some(apps) = Self::apps(caller).await? else {
			bevybail!(
				"this launch declares no stack keeping its state at AWS, so \
				there is no deployer to elevate"
			);
		};
		let session = AdminSession::acquire().await.map_err(|err| {
			bevyhow!(
				"an elevated deploy asks for the six-digit code from the phone, \
				and {err}\n\nrun it in a terminal: `{}`",
				ask.relay
			)
		})?;
		let admin = AwsAdmin(session.sdk_vars());
		let account = admin.account_id().await?;
		let user = Self::user_name()?;
		let mut report = vec![admin.ensure_user(&user).await?];
		for app in apps.iter() {
			// the boundaries FIRST: the deploy policy may only carry the
			// conditions naming them once they exist and every principal wears one
			for boundary in app.boundaries.values() {
				report.push(boundary.converge(&admin, &account).await?);
			}
			report.push(
				app.deploy_policy(&admin, &account)
					.await?
					.converge(&admin, &account, &user)
					.await?,
			);
		}
		let grants = Self::grant_lines(&apps)?;
		report.push(
			Self::converge_key(caller, &admin, &user, &apps, &grants, ask.roll)
				.await?,
		);
		CredentialElevation {
			report,
			env: match ask.run {
				true => session.sdk_vars(),
				false => Vec::new(),
			},
			cleanup: None,
		}
		.xok()
	}

	/// One [`AppPolicies`] per app this launch declares, lowered from every
	/// stack of it at the launch's stage and `prod`, in app order. Renders in
	/// one world pass, so a stack that cannot render fails here rather than at
	/// the account. `None` when this launch deploys nothing at AWS: no stack,
	/// or state kept locally, which deploys as nobody.
	async fn apps(caller: &AsyncEntity) -> Result<Option<Vec<AppPolicies>>> {
		let state_bucket = match terra::Project::resolve_backend(caller).await?
		{
			ResolvedBackend::S3(s3) => s3.bucket().clone(),
			ResolvedBackend::Local(_) => return None.xok(),
		};
		let rendered = caller
			.with_world(|world, _| DeployCredential::render_stages(world))
			.await??;
		if rendered.is_empty() {
			return None.xok();
		}
		let mut apps = BTreeMap::<SmolStr, AppPolicies>::new();
		for (stack, _deployment, config) in rendered.iter() {
			let app =
				apps.remove(stack.app_name())
					.unwrap_or_else(|| AppPolicies {
						deploy: DeployerPolicy::new(
							stack.app_name().clone(),
							state_bucket.clone(),
						),
						boundaries: BTreeMap::new(),
						principals: Vec::new(),
					});
			let mut principals = app.principals;
			principals.extend(RuntimeBoundary::principals(config)?);
			let mut boundaries = app.boundaries;
			let boundary = boundaries
				.remove(stack.stage())
				.unwrap_or_else(|| {
					RuntimeBoundary::new(
						stack.app_name().clone(),
						stack.stage().clone(),
					)
				})
				.lower(config)?;
			boundaries.insert(stack.stage().clone(), boundary);
			apps.insert(stack.app_name().clone(), AppPolicies {
				deploy: app.deploy.lower(stack, config)?,
				boundaries,
				principals,
			});
		}
		Some(apps.into_values().collect::<Vec<_>>()).xok()
	}

	/// The user every launch of this repo deploys as: the workspace directory,
	/// kebab-cased, and `-deployer`. See the type docs for why the repo rather
	/// than the app names it.
	pub(crate) fn user_name() -> Result<String> {
		let root = fs_ext::workspace_root();
		let name = root
			.file_name()
			.and_then(|name| name.to_str())
			.map(|name| name.replace('_', "-"))
			.filter(|name| {
				!name.is_empty()
					&& name
						.chars()
						.all(|char| char.is_ascii_alphanumeric() || char == '-')
			})
			.ok_or_else(|| {
				bevyhow!(
					"cannot name a deployer after the workspace directory \
					`{}`: an iam user name is alphanumeric",
					root.display()
				)
			})?;
		format!("{name}-deployer").xok()
	}

	/// The dry run's answer, needing no credential: the user and every policy
	/// document, pretty printed so it reads and pipes. The boundary conditions
	/// are absent, since whether they apply is an account read a dry run does
	/// not make.
	async fn describe_of(caller: &AsyncEntity) -> Result<String> {
		let Some(apps) = Self::apps(caller).await? else {
			return "aws: this launch declares no stack keeping its state at AWS\n"
				.to_string()
				.xok();
		};
		let mut out = format!("user {}\n", Self::user_name()?);
		for app in apps.iter() {
			out.push_str(&format!(
				"\npolicy {}\n{}\n",
				app.deploy.policy_name(),
				serde_json::to_string_pretty(&app.deploy.to_json())?
			));
			for boundary in
				app.boundaries.values().filter(|held| !held.is_empty())
			{
				out.push_str(&format!(
					"\npolicy {}\n{}\n",
					boundary.policy_name(),
					serde_json::to_string_pretty(&boundary.to_json())?
				));
			}
		}
		out.xok()
	}

	/// Converge the access key: mint one when the document holds none of this
	/// user's, or on `roll`, sealing the pair with `grants` before deleting
	/// what it replaces; restate a current pair's records otherwise. The key
	/// id's record keeps the grants of apps this launch does not declare.
	async fn converge_key(
		caller: &AsyncEntity,
		admin: &AwsAdmin,
		user: &str,
		apps: &[AppPolicies],
		grants: &[String],
		roll: bool,
	) -> Result<String> {
		let handle = SecretsHandle::resolve(caller, None).await?;
		let identity = AgeIdentityFile::require()?;
		let mut document = handle.read_or_new().await?;
		let opened = document.open(&identity).ok();
		let held_id = opened
			.as_ref()
			.and_then(|opened| opened.get(Self::KEY_ID).cloned());
		let held_secret = opened
			.as_ref()
			.and_then(|opened| opened.get(Self::KEY_SECRET).cloned());
		// another entry's apps keep their lines, this launch's are replaced
		let mut merged = DeployCredential::held_grants(held_id.as_ref())
			.into_iter()
			.filter(|line| !Self::belongs(apps, line))
			.map(String::from)
			.chain(grants.iter().cloned())
			.collect::<Vec<_>>();
		merged.sort();
		merged.dedup();
		let group = held_id
			.as_ref()
			.map(|secret| secret.group.to_string())
			.unwrap_or_else(|| Self::DEFAULT_GROUP.to_string());
		let held = held_id.as_ref().map(|secret| secret.value.clone());
		let existing = admin.access_keys(user).await?;
		let holds_active = held
			.as_ref()
			.is_some_and(|held| existing.iter().any(|key| key == held));
		if holds_active && !roll {
			let mut resealed = false;
			for (secret, record) in [
				(held_id.as_ref(), Self::id_record(user, &merged)),
				(held_secret.as_ref(), Self::secret_record(user)),
			] {
				if let Some((secret, record)) = secret.and_then(|secret| {
					DeployCredential::restated(secret, &group, record)
						.map(|record| (secret, record))
				}) {
					document.set(
						&identity,
						&group,
						&secret.name,
						&secret.value,
						record,
					)?;
					resealed = true;
				}
			}
			if resealed {
				handle.write(&document).await?;
			}
			return format!(
				"key {} of {user} is sealed in {}{}",
				held.unwrap_or_default(),
				handle.describe(),
				match resealed {
					true => ", its note, roll and grants resealed",
					false => "",
				}
			)
			.xok();
		}
		// a user may hold two keys at once, so anything this document does not
		// name goes before the mint rather than after it
		for stale in existing.iter().filter(|key| Some(*key) != held.as_ref()) {
			admin.delete_key(user, stale).await?;
			info!("deleted unreferenced key {stale} of {user}");
		}
		let (id, secret) = admin.create_key(user).await?;
		Self::prove_key(&id, &secret).await?;
		document.set(
			&identity,
			&group,
			Self::KEY_ID,
			&id,
			Self::id_record(user, &merged),
		)?;
		document.set(
			&identity,
			&group,
			Self::KEY_SECRET,
			&secret,
			Self::secret_record(user),
		)?;
		handle.write(&document).await?;
		// only now: the pair that replaces it is sealed and proven
		if let Some(held) =
			held.filter(|held| existing.iter().any(|key| key == held))
		{
			admin.delete_key(user, &held).await?;
			info!("deleted the replaced key {held} of {user}");
		}
		format!(
			"key {id} minted for {user} and sealed in {} (group `{group}`)",
			handle.describe()
		)
		.xok()
	}

	/// The key id's record: what it is, how it rolls, and the grants a plain
	/// deploy compares the declarations against.
	fn id_record(user: &str, grants: &[String]) -> SecretRecord {
		SecretRecord {
			role: Some(SecretRole::EnvVar),
			note: Some(
				format!(
					"aws deployer `{user}`: the access key id of this repo's \
					own iam user; the deploy verbs and every `aws s3 sync` \
					read the pair from here, never from ~/.aws"
				)
				.into(),
			),
			roll: Some(SecretRoll::Elevated),
			metadata: BTreeMap::from([(
				DeployCredential::GRANTS.into(),
				grants.join("\n").into(),
			)]),
			..default()
		}
	}

	/// The secret half's record.
	fn secret_record(user: &str) -> SecretRecord {
		SecretRecord {
			role: Some(SecretRole::EnvVar),
			note: Some(
				format!(
					"aws deployer `{user}`: the secret half of {}, shown once \
					at creation",
					Self::KEY_ID
				)
				.into(),
			),
			roll: Some(SecretRoll::Elevated),
			..default()
		}
	}

	/// Prove a freshly minted pair before anything relies on it: IAM is
	/// eventually consistent, so a new key answers `InvalidClientTokenId` for
	/// a few seconds. Retried rather than slept through, and a failure here
	/// leaves the old key in place.
	async fn prove_key(id: &str, secret: &str) -> Result {
		const ATTEMPTS: usize = 10;
		let mut last = None;
		for attempt in 0..ATTEMPTS {
			let result = aws_cli_ext::sts(["get-caller-identity"])
				.without_env("AWS_PROFILE")
				.without_env("AWS_SESSION_TOKEN")
				.with_env(Self::KEY_ID, id)
				.with_env(Self::KEY_SECRET, secret)
				.with_secret(secret)
				.run_async_stdout()
				.await;
			match result {
				Ok(_) => return OK,
				Err(err) => {
					last = Some(err);
					time_ext::sleep_secs(1 + attempt as u64 / 3).await;
				}
			}
		}
		bevybail!(
			"the minted key never answered `sts:GetCallerIdentity`, so it is \
			not sealed and the previous key is untouched: {}",
			last.map(|err| err.to_string()).unwrap_or_default()
		)
	}
}

/// The administrator session one elevated deploy acts as at IAM, handed to
/// every `aws` call explicitly: the launch's own environment holds the sealed
/// deployer pair, which is exactly the credential that cannot do this.
struct AwsAdmin(Vec<(SmolStr, SmolStr)>);

impl AwsAdmin {
	/// `process` as this session: its three variables set, an inherited
	/// profile dropped, every value redacted from any rendering of the command.
	fn wrap(&self, process: ChildProcess) -> ChildProcess {
		self.0
			.iter()
			.fold(process.without_env("AWS_PROFILE"), |process, (_, value)| {
				process.with_secret(value.clone())
			})
			.with_envs(self.0.clone())
	}

	/// This session's own account, which every policy arn composes with.
	async fn account_id(&self) -> Result<String> {
		self.wrap(aws_cli_ext::sts([
			"get-caller-identity",
			"--query",
			"Account",
			"--output",
			"text",
		]))
		.run_async_stdout()
		.await
		.map(|out| out.trim().to_string())
		.map_err(|err| {
			bevyhow!(
				"the administrator session does not answer \
				`sts:GetCallerIdentity`: {err}"
			)
		})
	}

	/// An `aws iam` call as this session.
	async fn iam<'a>(
		&self,
		args: impl IntoIterator<Item = &'a str>,
	) -> Result<String> {
		self.wrap(aws_cli_ext::iam(args)).run_async_stdout().await
	}

	/// [`iam`](Self::iam) answering json, `None` when the entity does not
	/// exist: every converge step is get-or-create, and absence is the one
	/// failure that is not one.
	async fn iam_json<'a>(
		&self,
		args: impl IntoIterator<Item = &'a str>,
	) -> Result<Option<Value>> {
		match self.iam(args).await {
			Ok(body) => serde_json::from_str::<Value>(&body).map(Some)?.xok(),
			Err(err) if err.to_string().contains("NoSuchEntity") => None.xok(),
			Err(err) => Err(err),
		}
	}

	/// The user, created when absent.
	async fn ensure_user(&self, user: &str) -> Result<String> {
		if self
			.iam_json(["get-user", "--user-name", user])
			.await?
			.is_some()
		{
			return format!("user {user} exists").xok();
		}
		self.iam([
			"create-user",
			"--user-name",
			user,
			"--tags",
			"Key=beet,Value=deployer",
		])
		.await?;
		format!("user {user} created").xok()
	}

	/// Every access key id of `user`, active or not: an inactive one still
	/// counts against the limit of two.
	async fn access_keys(&self, user: &str) -> Result<Vec<SmolStr>> {
		let body = self
			.iam([
				"list-access-keys",
				"--user-name",
				user,
				"--query",
				"AccessKeyMetadata[].AccessKeyId",
				"--output",
				"json",
			])
			.await?;
		serde_json::from_str::<Vec<SmolStr>>(&body)?.xok()
	}

	/// Mint a pair. Its stdout carries the only copy of the secret half, so
	/// nothing here logs the output.
	async fn create_key(&self, user: &str) -> Result<(SmolStr, SmolStr)> {
		let body = self
			.iam([
				"create-access-key",
				"--user-name",
				user,
				"--query",
				"AccessKey.[AccessKeyId,SecretAccessKey]",
				"--output",
				"json",
			])
			.await?;
		let pair = serde_json::from_str::<Vec<SmolStr>>(&body)?;
		match pair.as_slice() {
			[id, secret] => (id.clone(), secret.clone()).xok(),
			_ => bevybail!("`iam create-access-key` answered no pair"),
		}
	}

	async fn delete_key(&self, user: &str, id: &str) -> Result {
		self.iam([
			"delete-access-key",
			"--user-name",
			user,
			"--access-key-id",
			id,
		])
		.await
		.map(|_| ())
	}
}

impl AppPolicies {
	/// This app's deploy policy, carrying the boundary conditions only when
	/// every principal its stacks render ALREADY wears the boundary.
	///
	/// The order of operations, enforced rather than remembered: see
	/// [`AwsDeployer`]. An app with no principals at the account yet is
	/// conditioned from birth, since its `CreateRole` carries the boundary.
	async fn deploy_policy(
		&self,
		admin: &AwsAdmin,
		account: &str,
	) -> Result<DeployerPolicy> {
		if self.boundaries.values().all(RuntimeBoundary::is_empty) {
			return self.deploy.clone().xok();
		}
		// any boundary OF THIS APP satisfies the condition, so a principal
		// capped by a stage this launch did not render still passes
		let prefix =
			format!("arn:aws:iam::{account}:policy/{}--", self.deploy.app());
		let of_this_app = |arn: &str| {
			arn.starts_with(&prefix) && arn.ends_with(RuntimeBoundary::SUFFIX)
		};
		let mut uncapped = Vec::new();
		for (kind, name) in &self.principals {
			let Some(principal) = admin
				.iam_json([
					&format!("get-{kind}"),
					&format!("--{kind}-name"),
					name.as_str(),
				])
				.await?
			else {
				// not at the account yet, so its `CreateRole` will carry the
				// boundary and there is nothing to wait for
				continue;
			};
			let held = principal
				.get(match *kind {
					"role" => "Role",
					_ => "User",
				})
				.and_then(|principal| principal.get("PermissionsBoundary"))
				.and_then(|boundary| boundary.get("PermissionsBoundaryArn"))
				.and_then(Value::as_str);
			if !held.is_some_and(of_this_app) {
				uncapped.push(format!("{kind} {name}"));
			}
		}
		if !uncapped.is_empty() {
			warn!(
				"{} principal(s) of `{}` carry no boundary of this app yet, so \
				its deploy policy stays unconditioned: this deploy caps them, \
				and the next elevated deploy closes the door ({})",
				uncapped.len(),
				self.deploy.policy_name(),
				uncapped.join(", ")
			);
			return self.deploy.clone().xok();
		}
		self.deploy.clone().with_boundary().xok()
	}
}

impl RuntimeBoundary {
	/// Converge this boundary at the account: the same get-or-create-or-update
	/// as a deploy policy, attached to nothing, since a boundary is named by
	/// the principals that wear it rather than attached to a user.
	async fn converge(
		&self,
		admin: &AwsAdmin,
		account: &str,
	) -> Result<String> {
		if self.is_empty() {
			return format!(
				"{} renders no principal, so it needs no boundary",
				self.policy_name()
			)
			.xok();
		}
		DeployerPolicy::converge_document(
			admin,
			account,
			&self.policy_name(),
			&self.to_json(),
		)
		.await
	}
}

impl DeployerPolicy {
	/// Converge this policy at the account and attach it to `user`: created
	/// when absent, a new default version when its document changed, and
	/// nothing at all when it already matches.
	async fn converge(
		&self,
		admin: &AwsAdmin,
		account: &str,
		user: &str,
	) -> Result<String> {
		let report = Self::converge_document(
			admin,
			account,
			&self.policy_name(),
			&self.to_json(),
		)
		.await?;
		Self::attach(
			admin,
			&format!("arn:aws:iam::{account}:policy/{}", self.policy_name()),
			user,
		)
		.await?;
		report.xok()
	}

	/// Converge one managed policy document at the account: created when
	/// absent, a new default version when it changed, nothing when it matches.
	/// Shared with [`RuntimeBoundary`], which is the same act minus the
	/// attachment, since a boundary is worn rather than attached.
	async fn converge_document(
		admin: &AwsAdmin,
		account: &str,
		name: &str,
		document: &Value,
	) -> Result<String> {
		// a managed policy is capped at 6144 characters, and the failure is a
		// `LimitExceeded` mid-converge with earlier apps already converged
		const LIMIT: usize = 6144;
		let rendered = document.to_string();
		if rendered.len() > LIMIT {
			bevybail!(
				"the document for `{name}` is {} characters, over IAM's {LIMIT} \
				limit for a managed policy: the app declares more than one \
				policy can express, so split the stack or narrow its grants",
				rendered.len()
			);
		}
		let arn = format!("arn:aws:iam::{account}:policy/{name}");
		let existing = admin
			.iam_json([
				"get-policy",
				"--policy-arn",
				arn.as_str(),
				"--query",
				"Policy.DefaultVersionId",
				"--output",
				"json",
			])
			.await?;
		match existing {
			None => {
				admin
					.iam([
						"create-policy",
						"--policy-name",
						name,
						"--policy-document",
						rendered.as_str(),
					])
					.await?;
				format!("policy {name} created").xok()
			}
			Some(version) => {
				Self::converge_version(
					admin,
					name,
					&arn,
					document,
					version.as_str().unwrap_or_default(),
				)
				.await
			}
		}
	}

	/// Replace the default version when the live document differs, pruning the
	/// oldest non-default version first: a managed policy holds five.
	async fn converge_version(
		admin: &AwsAdmin,
		name: &str,
		arn: &str,
		document: &Value,
		default_version: &str,
	) -> Result<String> {
		let live = admin
			.iam([
				"get-policy-version",
				"--policy-arn",
				arn,
				"--version-id",
				default_version,
				"--query",
				"PolicyVersion.Document",
				"--output",
				"json",
			])
			.await?;
		// the cli decodes the url-encoded document, so this compares values
		// rather than text: a key order or a whitespace change is not a change
		if serde_json::from_str::<Value>(&live).ok().as_ref() == Some(document)
		{
			return format!("policy {name} unchanged").xok();
		}
		let versions = admin
			.iam([
				"list-policy-versions",
				"--policy-arn",
				arn,
				"--query",
				"Versions[?IsDefaultVersion==`false`].VersionId",
				"--output",
				"json",
			])
			.await?
			.xmap(|body| serde_json::from_str::<Vec<SmolStr>>(&body))?;
		if versions.len() >= 4 {
			let oldest = versions.last().cloned().unwrap_or_default();
			admin
				.iam([
					"delete-policy-version",
					"--policy-arn",
					arn,
					"--version-id",
					oldest.as_str(),
				])
				.await?;
			info!("pruned version {oldest} of {name}, which holds five");
		}
		admin
			.iam([
				"create-policy-version",
				"--policy-arn",
				arn,
				"--policy-document",
				document.to_string().as_str(),
				"--set-as-default",
			])
			.await?;
		format!("policy {name} updated").xok()
	}

	/// Attach the policy to `user` unless it already is.
	async fn attach(admin: &AwsAdmin, arn: &str, user: &str) -> Result {
		let attached = admin
			.iam([
				"list-attached-user-policies",
				"--user-name",
				user,
				"--query",
				"AttachedPolicies[].PolicyArn",
				"--output",
				"json",
			])
			.await?
			.xmap(|body| serde_json::from_str::<Vec<SmolStr>>(&body))?;
		if attached.iter().any(|held| held == arn) {
			return OK;
		}
		admin
			.iam([
				"attach-user-policy",
				"--user-name",
				user,
				"--policy-arn",
				arn,
			])
			.await
			.map(|_| ())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The user is the workspace directory, kebab-cased: in this checkout,
	/// beet's own.
	#[beet_core::test]
	fn names_the_user_after_the_repo() {
		AwsDeployer::user_name()
			.unwrap()
			.as_str()
			.xpect_contains("-deployer")
			.xnot()
			.xpect_contains("_");
	}

	/// Outside the disposable stage, removing a bucket, a table, a database or
	/// a data volume is protected, and so is any write to a bucket's versioning
	/// or lifecycle; an update of anything else is not, and `dev` is torn down
	/// every cycle.
	#[beet_core::test]
	fn protects_only_the_irreversible() {
		let change = |kind: &str, action: &str| tofu::PlannedChange {
			address: format!("{kind}.x").into(),
			resource_type: kind.into(),
			action: action.into(),
		};
		let prod = Stack::new("app").with_stage("prod").resolve(&default());
		let dev = Stack::new("app")
			.with_stage(BootstrapConfig::DEFAULT_STAGE)
			.resolve(&default());
		AwsDeployer
			.protects(&change("aws_s3_bucket", "delete"), &prod)
			.xpect_some();
		AwsDeployer
			.protects(&change("aws_dynamodb_table", "replace"), &prod)
			.xpect_some();
		AwsDeployer
			.protects(&change("aws_s3_bucket", "update"), &prod)
			.xpect_none();
		AwsDeployer
			.protects(&change("aws_s3_bucket", "delete"), &dev)
			.xpect_none();
		AwsDeployer
			.protects(&change("aws_instance", "delete"), &prod)
			.xpect_none();
		AwsDeployer
			.protects(&change("aws_ebs_volume", "replace"), &prod)
			.xpect_some();
		AwsDeployer
			.protects(&change("aws_ebs_volume", "update"), &prod)
			.xpect_none();
		for kind in [
			"aws_s3_bucket_versioning",
			"aws_s3_bucket_lifecycle_configuration",
		] {
			AwsDeployer
				.protects(&change(kind, "create"), &prod)
				.xpect_some();
			AwsDeployer
				.protects(&change(kind, "update"), &prod)
				.xpect_some();
			AwsDeployer
				.protects(&change(kind, "update"), &dev)
				.xpect_none();
		}
	}
}

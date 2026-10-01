//! `deployer/mint`: this repo's own AWS deployer user, its policies and the
//! pair its credential document holds.

use crate::actions::aws_cli_ext;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;

/// Request params for [`DeployerMint`], surfaced in `--help`.
#[derive(Reflect)]
struct MintParams {
	/// Print the user and the policy documents, WRITING to neither the account
	/// nor the document. The documents are the response, so a one-shot pipes
	/// them.
	///
	/// It still READS: the stacks render first, which resolves the state
	/// backend, so this needs a credential that answers
	/// `sts:GetCallerIdentity` even though it needs no administrator.
	///
	/// And the documents it prints are a FLOOR rather than the whole: whether a
	/// deploy policy may carry the boundary conditions is an account read this
	/// skips, so a preview shows `IrreversibleNeedsMfa` and never
	/// `IamOnlyUnderTheBoundary`, `NeverUncap` or `NeverMintOrRewriteACap`.
	/// Read a live policy to see those.
	dry_run: bool,
	/// Mint a fresh access key even when the document already holds a working
	/// one, and delete the key it replaces: this is the rotation.
	rotate: bool,
	/// The document group the pair is sealed in, `default` when absent.
	group: Option<String>,
}

/// `<DeployerMint/>` — converge the AWS deployer this repo deploys as: one IAM
/// user for the credential document, one managed policy per app it declares
/// ([`DeployerPolicy`], lowered from what the stacks render), and one access
/// key, sealed into the document as `AWS_ACCESS_KEY_ID` and
/// `AWS_SECRET_ACCESS_KEY`.
///
/// ```sh
/// beet deployer/mint --dry-run   # the policies, nothing touched
/// beet deployer/mint             # converge the user, its policies and its key
/// beet deployer/mint --rotate    # ..and replace the key even if it works
/// ```
///
/// ## Why a verb rather than a block
///
/// Everything else about a stack is declared, and a deployer user could be:
/// `aws_iam_user` and `aws_iam_access_key` are bound, so the apply that
/// provisions a stack could provision the identity applying it. It should not.
/// A deployer declared in its own stack has to hold `iam:PutUserPolicy` on its
/// own user to plan, which is an administrator with extra steps; the stack's
/// `destroy` would delete the credential it is authenticating with; and the
/// tier cannot close, since the apply that mints the first deployer has to run
/// as some earlier one. So the identity lives OUTSIDE tofu, and what the block
/// would have bought — a policy derived from the declarations rather than
/// written by hand — is bought here instead, from the same render.
///
/// ## Which credential it runs as
///
/// Minting an IAM user is an administrator's act, so this verb is the one that
/// does not run as the deployer it converges: its scoped policy grants no
/// `iam:` action over itself. The launch loads the document's records into the
/// environment but an existing variable WINS, so an admin pair is passed for
/// one command:
///
/// ```sh
/// AWS_ACCESS_KEY_ID=.. AWS_SECRET_ACCESS_KEY=.. beet deployer/mint
/// ```
///
/// The first mint is the exception: the document still holds whatever pair
/// deployed the repo until now, so it runs with no environment at all and
/// replaces that pair with the scoped one.
///
/// ## One stage at a time
///
/// A launch renders the stacks of ONE stage, so the services a policy grants
/// are that stage's. Every NAME it scopes is stage-independent
/// ([`DeployerPolicy`]), so the choice only ever costs a missing service: mint
/// under the stage that actually deploys (`--stage=prod` where that is not the
/// default), which this warns about when it is not.
///
/// ## The order of operations, which this enforces
///
/// Per app it converges TWO managed policies: `<app>--runtime-boundary`
/// ([`RuntimeBoundary`], the ceiling on what a deploy may create) and
/// `<app>--deploy` ([`DeployerPolicy`], what the operator may do). The
/// boundary goes first, and the deploy policy carries the conditions that name
/// it only once every principal the render names already WEARS it, which this
/// asks the account rather than taking on trust.
///
/// That is not caution, it is the only order that works: a `StringNotEquals`
/// deny on `iam:PermissionsBoundary` is true for a principal carrying no
/// boundary, so conditioning the policy before the apply that attaches the
/// first boundaries would deny that apply and block every deploy of the app
/// until an administrator intervened. So the first mint of an app with live
/// uncapped principals writes an unconditioned policy and says so; the apply
/// caps them; the next mint closes the door. An app whose principals are not
/// at the account yet is conditioned from birth.
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
///   its own entry.
/// - an access key, minted when the document holds none of this user's, or on
///   `--rotate`. The pair is sealed before the key it replaces is deleted, and
///   the new one is proven with `sts:GetCallerIdentity` before either.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(
	PathPartial = PathPartial::new("deployer/mint"),
	ParamsPartial = ParamsPartial::new::<MintParams>()
)]
pub async fn DeployerMint(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<MintParams>()?;
	let apps = DeployerMint::apps(&cx.caller).await?;
	let user = DeployerMint::user_name()?;
	if params.dry_run {
		return Response::ok_text(DeployerMint::describe(&user, &apps)?).xok();
	}
	let account = DeployerMint::account_id().await?;
	let mut report = vec![DeployerMint::ensure_user(&user).await?];
	for app in apps.iter() {
		// the boundaries FIRST: the deploy policy may only carry the
		// conditions naming them once they exist and every principal wears one
		for boundary in app.boundaries.values() {
			report.push(boundary.converge(&account).await?);
		}
		report.push(
			app.deploy_policy(&account)
				.await?
				.converge(&account, &user)
				.await?,
		);
	}
	report.push(DeployerMint::converge_key(&cx.caller, &user, &params).await?);
	Response::ok_text(format!("{}\n", report.join("\n"))).xok()
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

impl DeployerMint {
	/// The `--group` default, the group a first mint creates.
	const DEFAULT_GROUP: &'static str = SecretsDocument::DEFAULT_GROUP;

	/// The record names the pair is sealed under, which are the sdk's own
	/// variables: what the document holds is what the environment gets.
	const KEY_ID: &'static str = "AWS_ACCESS_KEY_ID";
	const KEY_SECRET: &'static str = "AWS_SECRET_ACCESS_KEY";

	/// One [`AppPolicies`] per app this launch declares, lowered from every
	/// stack of it, in app order. Renders in one world pass, so a stack that
	/// cannot render fails here rather than at the account.
	async fn apps(caller: &AsyncEntity) -> Result<Vec<AppPolicies>> {
		let state_bucket = match terra::Project::resolve_backend(caller).await?
		{
			ResolvedBackend::S3(s3) => s3.bucket().clone(),
			ResolvedBackend::Local(local) => bevybail!(
				"this launch keeps its state locally ({}), so it deploys as \
				nobody and needs no deployer: declare an `<S3StateBackend/>` \
				or run this from an entry that does",
				local.uri()
			),
		};
		let rendered = caller
			.with_world(|world, _| {
				RenderScope::render_all(world)?
					.into_iter()
					.map(RenderScope::finish)
					.collect::<Result<Vec<_>>>()
			})
			.await??;
		if rendered.is_empty() {
			bevybail!(
				"this launch declares no `<Stack>`, so there is nothing for a \
				deployer to be allowed to do"
			);
		}
		// one launch renders one stage, and a stage may declare resources
		// another does not, so a policy derived from a non-prod launch can be
		// short of what the deployed stage needs. A stack with a PINNED stage
		// renders the same whatever the launch says, so only a stack that took
		// the launch's own stage is worth warning about.
		let launch = BootstrapConfig::get().stage.clone();
		if launch != BootstrapConfig::PROD_STAGE
			&& let Some(stage) = rendered
				.iter()
				.map(|(stack, ..)| stack.stage())
				.find(|stage| **stage == launch)
		{
			warn!(
				"lowering the stage `{stage}` declarations: a stage that \
				declares more (a certificate, a custom domain) needs its own \
				run, so mint under the stage that deploys \
				(`--stage={}`)",
				BootstrapConfig::PROD_STAGE
			);
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
		apps.into_values().collect::<Vec<_>>().xok()
	}

	/// The user every launch of this repo deploys as: the workspace directory,
	/// kebab-cased, and `-deployer`. See the type docs for why the repo rather
	/// than the app names it.
	fn user_name() -> Result<String> {
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

	/// The dry run's answer: the user and every policy document, pretty
	/// printed so it reads and pipes. The boundary conditions are absent,
	/// since whether they apply is an account read a dry run does not make.
	fn describe(user: &str, apps: &[AppPolicies]) -> Result<String> {
		let mut out = format!("user {user}\n");
		for app in apps {
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

	/// This caller's own account, which every policy arn composes with. Over
	/// the cli rather than the sdk so the whole verb reaches AWS one way, and
	/// it is the credential check too: a launch with no usable pair fails here
	/// rather than at the first IAM call.
	async fn account_id() -> Result<String> {
		aws_cli_ext::sts([
			"get-caller-identity",
			"--query",
			"Account",
			"--output",
			"text",
		])
		.run_async_stdout()
		.await
		.map(|out| out.trim().to_string())
		.map_err(|err| {
			bevyhow!(
				"no aws credentials answer `sts:GetCallerIdentity`, so there \
				is nobody to mint as: {err}"
			)
		})
	}

	/// The user, created when absent.
	async fn ensure_user(user: &str) -> Result<String> {
		if Self::iam_json(["get-user", "--user-name", user])
			.await?
			.is_some()
		{
			return format!("user {user} exists").xok();
		}
		Self::iam([
			"create-user",
			"--user-name",
			user,
			"--tags",
			"Key=beet,Value=deployer",
		])
		.await?;
		format!("user {user} created").xok()
	}

	/// An `aws iam` call. Every one of them is an administrator's, so an
	/// access-denied failure is re-raised as what to do about it: the deployer
	/// this converges holds no `iam:` action, deliberately, so running the verb
	/// as it is the expected mistake.
	async fn iam<'a>(
		args: impl IntoIterator<Item = &'a str>,
	) -> Result<String> {
		aws_cli_ext::iam(args)
			.run_async_stdout()
			.await
			.map_err(|err| match err.to_string().contains("AccessDenied") {
				true => bevyhow!(
					"this credential is no administrator, and a deployer is \
					never one: pass an admin pair for this one command \
					(`AWS_ACCESS_KEY_ID=.. AWS_SECRET_ACCESS_KEY=.. beet \
					deployer/mint`), which wins over the document. {err}"
				),
				false => err,
			})
	}

	/// [`iam`](Self::iam) answering json, `None` when the entity does not
	/// exist: every converge step below is get-or-create, and absence is the
	/// one failure that is not one.
	async fn iam_json<'a>(
		args: impl IntoIterator<Item = &'a str>,
	) -> Result<Option<Value>> {
		match Self::iam(args).await {
			Ok(body) => serde_json::from_str::<Value>(&body).map(Some)?.xok(),
			Err(err) if err.to_string().contains("NoSuchEntity") => None.xok(),
			Err(err) => Err(err),
		}
	}

	/// Converge the access key: mint one when the document holds none of this
	/// user's, or on `--rotate`, sealing the pair before deleting what it
	/// replaces.
	async fn converge_key(
		caller: &AsyncEntity,
		user: &str,
		params: &MintParams,
	) -> Result<String> {
		let handle = SecretsHandle::resolve(caller, None).await?;
		let identity = AgeIdentityFile::require()?;
		let mut document = handle.read_or_new().await?;
		let held = document.open(&identity).ok().and_then(|opened| {
			opened.get(Self::KEY_ID).map(|secret| secret.value.clone())
		});
		let existing = Self::access_keys(user).await?;
		let holds_active = held
			.as_ref()
			.is_some_and(|held| existing.iter().any(|key| key == held));
		if holds_active && !params.rotate {
			return format!(
				"key {} of {user} is already sealed in {}, `--rotate` mints \
				another",
				held.unwrap_or_default(),
				handle.describe()
			)
			.xok();
		}
		// a user may hold two keys at once, so anything this document does not
		// name goes before the mint rather than after it
		for stale in existing.iter().filter(|key| Some(*key) != held.as_ref()) {
			Self::delete_key(user, stale).await?;
			info!("deleted unreferenced key {stale} of {user}");
		}
		let (id, secret) = Self::create_key(user).await?;
		Self::prove_key(&id, &secret).await?;
		let group = params.group.as_deref().unwrap_or(Self::DEFAULT_GROUP);
		document.set(&identity, group, Self::KEY_ID, &id, SecretRecord {
			role: Some(SecretRole::EnvVar),
			note: Some(
				format!(
					"aws deployer `{user}`: the access key id of this repo's \
					own iam user; the deploy verbs and every `aws s3 sync` \
					read the pair from here, never from ~/.aws"
				)
				.into(),
			),
			rotation: Some(Self::rotation()),
			..default()
		})?;
		document.set(
			&identity,
			group,
			Self::KEY_SECRET,
			&secret,
			SecretRecord {
				role: Some(SecretRole::EnvVar),
				note: Some(
					format!(
						"aws deployer `{user}`: the secret half of \
						{}, shown once at creation",
						Self::KEY_ID
					)
					.into(),
				),
				rotation: Some(Self::rotation()),
				..default()
			},
		)?;
		handle.write(&document).await?;
		// only now: the pair that replaces it is sealed and proven
		if let Some(held) =
			held.filter(|held| existing.iter().any(|key| key == held))
		{
			Self::delete_key(user, &held).await?;
			info!("deleted the replaced key {held} of {user}");
		}
		format!(
			"key {id} minted for {user} and sealed in {} (group `{group}`)",
			handle.describe()
		)
		.xok()
	}

	/// How the pair rotates: this verb again, which is only an
	/// administrator's to run.
	fn rotation() -> SecretRotation {
		SecretRotation::manual(
			"beet deployer/mint --rotate\n> with an admin pair in the \
			environment, which wins over this document: \
			`AWS_ACCESS_KEY_ID=.. AWS_SECRET_ACCESS_KEY=.. beet \
			deployer/mint --rotate`\n> both halves are minted, sealed and \
			proven together, then the old key is deleted",
		)
	}

	/// Every access key id of `user`, active or not: an inactive one still
	/// counts against the limit of two.
	async fn access_keys(user: &str) -> Result<Vec<SmolStr>> {
		let body = Self::iam([
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
	async fn create_key(user: &str) -> Result<(SmolStr, SmolStr)> {
		let body = Self::iam([
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

	async fn delete_key(user: &str, id: &str) -> Result {
		Self::iam([
			"delete-access-key",
			"--user-name",
			user,
			"--access-key-id",
			id,
		])
		.await
		.map(|_| ())
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
				.with_env(Self::KEY_ID, id)
				.with_env(Self::KEY_SECRET, secret)
				.without_env("AWS_SESSION_TOKEN")
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

impl AppPolicies {
	/// This app's deploy policy, carrying the boundary conditions only when
	/// every principal its stacks render ALREADY wears the boundary.
	///
	/// The order of operations, enforced rather than remembered. A
	/// `StringNotEquals` deny on `iam:PermissionsBoundary` is true for a
	/// principal that carries none, so conditioning the policy before the
	/// apply that attaches the first boundaries would deny that apply and
	/// block every deploy of the app until an administrator intervened. So the
	/// first mint of an app with live uncapped principals writes the boundary
	/// and an unconditioned policy, the apply caps them, and the next mint
	/// closes the door behind it. An app with no principals at the account yet
	/// is conditioned from birth, since its `CreateRole` carries the boundary.
	async fn deploy_policy(&self, account: &str) -> Result<DeployerPolicy> {
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
			let Some(principal) = DeployerMint::iam_json([
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
				its deploy policy stays unconditioned: apply the stack to cap \
				them, then mint again to close the door ({})",
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
	async fn converge(&self, account: &str) -> Result<String> {
		if self.is_empty() {
			return format!(
				"{} renders no principal, so it needs no boundary",
				self.policy_name()
			)
			.xok();
		}
		DeployerPolicy::converge_document(
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
	async fn converge(&self, account: &str, user: &str) -> Result<String> {
		let report = Self::converge_document(
			account,
			&self.policy_name(),
			&self.to_json(),
		)
		.await?;
		Self::attach(
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
		account: &str,
		name: &str,
		document: &Value,
	) -> Result<String> {
		// a managed policy is capped at 6144 characters, and the failure is a
		// `LimitExceeded` mid-mint with earlier apps already converged
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
		let existing = DeployerMint::iam_json([
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
				DeployerMint::iam([
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
		name: &str,
		arn: &str,
		document: &Value,
		default_version: &str,
	) -> Result<String> {
		let live = DeployerMint::iam([
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
		let versions = DeployerMint::iam([
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
			DeployerMint::iam([
				"delete-policy-version",
				"--policy-arn",
				arn,
				"--version-id",
				oldest.as_str(),
			])
			.await?;
			info!("pruned version {oldest} of {name}, which holds five");
		}
		DeployerMint::iam([
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
	async fn attach(arn: &str, user: &str) -> Result {
		let attached = DeployerMint::iam([
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
		DeployerMint::iam([
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
		DeployerMint::user_name()
			.unwrap()
			.as_str()
			.xpect_contains("-deployer")
			.xnot()
			.xpect_contains("_");
	}
}

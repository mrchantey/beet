//! `<AwsDeployCredential/>`: an entry's own AWS deploy user, its policies and
//! the pair its secrets document holds.

use crate::actions::aws_cli_ext;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;

/// `<AwsDeployCredential/>`: the entry's AWS deploy credential, declared at
/// the entry's root like `<Secrets/>`: one IAM user for the entry, one managed
/// policy per app the entry declares ([`AwsDeployPolicy`], lowered from what
/// the stacks render), one runtime boundary per app and stage
/// ([`RuntimeBoundary`]), and one access key, sealed into the entry's secrets
/// document as `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` with the
/// policies it was converged against and the entry owning it in the key id's
/// record.
///
/// Nobody mints it by hand: a deploy checks the sealed grants against the
/// declarations before it writes anything and refuses naming `--elevated`
/// when they fall short, and the elevated deploy ([`DeployGate`]) converges
/// it here, as an administrator session bought with the code from the phone.
///
/// ## Why derived outside tofu rather than declared in a stack
///
/// Everything else about a stack is declared, and a deploy user could be:
/// `aws_iam_user` and `aws_iam_access_key` are bound, so the apply that
/// provisions a stack could provision the identity applying it. It should not.
/// A deploy user declared in its own stack has to hold `iam:PutUserPolicy` on
/// its own user to plan, which is an administrator with extra steps; the
/// stack's `destroy` would delete the credential it is authenticating with; and
/// the tier cannot close, since the apply that mints the first deploy user has
/// to run as some earlier one. So the identity lives OUTSIDE tofu, and what the
/// block would have bought (a policy derived from the declarations rather than
/// written by hand) is bought here instead, from the same render.
///
/// ## Which credential an elevated deploy runs this as
///
/// Minting an IAM user is an administrator's act, so the converge does not run
/// as the credential it converges: its scoped policy grants no `iam:` action
/// over itself. It runs as an administrator session
/// ([`AdminSession::acquire`]), the live one `beet admin` left on tmpfs when it
/// has time to spare, else an hour bought with the code from the phone, kept
/// nowhere.
///
/// ## Every stage that deploys
///
/// A launch renders the stacks of one stage, and a credential is per entry, so
/// the policies are lowered at the launch's stage AND at `prod`
/// ([`DeployCredential::render_stages`]): an elevated `dev` deploy can never
/// narrow what prod needs. The reverse holds for the sealed record: a prod
/// launch renders prod alone, so it keeps the dev boundary's grant as the
/// last dev launch sealed it rather than dropping a line it never rendered.
///
/// ## The order of operations, which this enforces
///
/// Per app it converges TWO kinds of managed policy: `<app>--<stage>--runtime-boundary`
/// ([`RuntimeBoundary`], the ceiling on what a deploy may create) and
/// `<app>--deploy` ([`AwsDeployPolicy`], what the deploy user may do). The
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
/// - the user, `<app_name>-deploy` ([`DeployCredential::name`]), created when
///   absent. Named for the ENTRY rather than for one of its apps, since its
///   secrets document holds one `AWS_ACCESS_KEY_ID` however many apps the
///   entry deploys, and belongs to that entry alone.
/// - one managed policy per app, attached to the user, rewritten only when its
///   document changed (managed rather than inline: a user's inline budget is
///   2048 characters for ALL of its policies together, which three apps of an
///   entry exceed). A policy attached to the user that is no app's of this
///   entry is detached, and deleted when nothing else holds it, so the key
///   holds exactly what its entry declares.
/// - each service-linked role the stacks need at the account
///   ([`AwsDeployPolicy::service_linked_roles`]), created when absent: the key
///   may never create one, and an account that has never held, ie, an API
///   Gateway custom domain lacks it. The key id's grants record each as
///   `service-linked-role <service>`, so a plain deploy into an account the
///   converge has not prepared refuses before it writes.
/// - an access key, minted when the document holds none of this user's, or on
///   `--roll`. The pair is sealed before the key it replaces is deleted, and
///   the new one is proven with `sts:GetCallerIdentity` before either. A
///   current pair's records are resealed, values unchanged, when their note,
///   roll, grants or owner are not what this writes now.
/// - the user the record held a key of before, whatever its name: once the
///   new pair is sealed, a deploy user this converge made (tagged as one)
///   other than the entry's own is retired, every policy detached (and deleted
///   when nothing else holds it), its keys deleted, then the user, so a renamed
///   credential leaves nothing behind.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct AwsDeployCredential;

impl DeployCredentialProvider for AwsDeployCredential {
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
				"this deploy would {} `{}`, which no stored deploy credential \
				may outside the `{}` stage: it is a change the credential is \
				denied without the code from the phone, since it can make data \
				final",
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
	deploy: AwsDeployPolicy,
	/// Per STAGE, since a boundary's content is one stage's names. The deploy
	/// policy is per app, and its condition matches any of them.
	boundaries: BTreeMap<SmolStr, RuntimeBoundary>,
	/// `(kind, name)` per principal, ie `("role", "beet-site--prod--..")`.
	principals: Vec<(&'static str, SmolStr)>,
}

/// Where a converged key lives: the account's id, and the member of the
/// Organization it is when an `<AwsAccount/>` is declared.
struct KeyHome<'a> {
	account: &'a str,
	member: Option<&'a MemberAccount>,
}

impl AwsDeployCredential {
	/// The record names the pair is sealed under, which are the sdk's own
	/// variables: what the document holds is what the environment gets.
	const KEY_ID: &'static str = "AWS_ACCESS_KEY_ID";
	const KEY_SECRET: &'static str = "AWS_SECRET_ACCESS_KEY";

	/// The key id record's metadata naming the account the user lives in.
	const ACCOUNT: &'static str = "account";
	/// ..and that account's name in the Organization, absent for the
	/// management account itself.
	const ACCOUNT_NAME: &'static str = "account_name";
	/// Stands in for a member account's id before the key lives there, which
	/// only the elevated deploy learns, as [`AwsAccount`]'s own description
	/// writes it.
	const UNKNOWN_ACCOUNT: &'static str = "<id>";

	/// The grant line naming a service-linked role the account must hold, ie
	/// `service-linked-role ops.apigateway.amazonaws.com`: not a policy the
	/// key holds but one the converge made sure of, recorded so a plain deploy
	/// into an account without it refuses before it writes.
	const SERVICE_LINKED_LINE: &'static str = "service-linked-role";

	/// The group a first converge seals into; a record already sealed stays
	/// in its own.
	const DEFAULT_GROUP: &'static str = SecretsDocument::DEFAULT_GROUP;

	/// The types whose removal [`AwsDeployPolicy`] denies the user without
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
	/// record keeps the boundary of a stage this launch did not render
	/// ([`Self::keeps`]), which is no line to drop.
	async fn status_of(
		&self,
		caller: &AsyncEntity,
	) -> Result<CredentialStatus> {
		// a key not yet in the declared account is stale for living elsewhere,
		// below, and one inside resolves its own state bucket
		let Some(apps) =
			Self::apps(caller, Self::moving_to(caller).await?).await?
		else {
			return CredentialStatus::not_needed().xok();
		};
		let grants = Self::grant_lines(&apps)?;
		let held = DeployCredential::sealed(caller, Self::KEY_ID).await;
		let rendered = |line: &str| !Self::keeps(&apps, line);
		let mut status = CredentialStatus::compare(
			self.id(),
			Self::KEY_ID,
			&grants,
			held.as_ref(),
			Some(&rendered),
		);
		// a credential in another account holds none of what this one needs
		if let Some((account, _)) = AwsAccount::declared(caller).await?
			&& let Some(held) = held.as_ref()
		{
			status
				.stale
				.extend(Self::elsewhere(held, &account.resolved_name()?));
		}
		status.xok()
	}

	/// The account to lower the policies against when the sealed key is absent
	/// or lives outside the declared account: [`Self::UNKNOWN_ACCOUNT`], since
	/// only the elevated deploy learns a new member's id, and the launch's own
	/// credential (none, on an entry's first deploy) cannot resolve it. `None`
	/// lowers against the account the launch runs as.
	async fn moving_to(caller: &AsyncEntity) -> Result<Option<&'static str>> {
		let Some((account, _)) = AwsAccount::declared(caller).await? else {
			return None.xok();
		};
		let name = account.resolved_name()?;
		DeployCredential::sealed(caller, Self::KEY_ID)
			.await
			.is_none_or(|held| Self::elsewhere(&held, &name).is_some())
			.then_some(Self::UNKNOWN_ACCOUNT)
			.xok()
	}

	/// Why the sealed key id `held` is not the declared account `name`'s,
	/// `None` when it lives there.
	fn elsewhere(held: &Secret, name: &str) -> Option<String> {
		let lives = held.record.metadata.get(Self::ACCOUNT_NAME);
		(lives.map(SmolStr::as_str) != Some(name)).then(|| {
			format!(
				"aws: the sealed `{}` lives in {}, not the declared account \
				`{name}`",
				Self::KEY_ID,
				lives
					.map(|lives| format!("`{lives}`"))
					.unwrap_or_else(|| "the management account".into())
			)
		})
	}

	/// Whether a held grant `line` is one a converge keeps without rendering
	/// it: the boundary of one of this entry's `apps` at a stage this launch
	/// did not render. A prod launch renders prod alone, so dev's boundary
	/// stays as a dev launch sealed it; every other line is this launch's to
	/// judge and replace.
	fn keeps(apps: &[AppPolicies], line: &str) -> bool {
		let name = line.split_once(' ').map_or(line, |(name, _)| name);
		apps.iter().any(|app| {
			name.strip_prefix(app.deploy.app().as_str())
				.and_then(|rest| rest.strip_prefix("--"))
				.and_then(|rest| rest.strip_suffix(RuntimeBoundary::SUFFIX))
				.and_then(|stage| stage.strip_suffix("--"))
				.is_some_and(|stage| {
					!stage.is_empty()
						&& !stage.contains("--")
						&& !app.boundaries.contains_key(stage)
				})
		})
	}

	/// The services whose service-linked role any of `apps` needs at the
	/// account ([`AwsDeployPolicy::service_linked_roles`]).
	fn service_linked_roles(apps: &[AppPolicies]) -> BTreeSet<&'static str> {
		apps.iter()
			.flat_map(|app| app.deploy.service_linked_roles().iter().copied())
			.collect()
	}

	/// The grants a converge seals: this launch's `grants`, beside every
	/// `held` line it keeps without rendering ([`Self::keeps`]), as that line
	/// was sealed.
	fn with_kept(
		held: Vec<SmolStr>,
		apps: &[AppPolicies],
		grants: &[String],
	) -> Vec<String> {
		let mut sealed = held
			.into_iter()
			.filter(|line| Self::keeps(apps, line))
			.map(String::from)
			.chain(grants.iter().cloned())
			.collect::<Vec<_>>();
		sealed.sort();
		sealed.dedup();
		sealed
	}

	/// Whether a policy detached from a deploy user is deleted: when the user
	/// was its one attachment of `attachments`, counted before the detach. An
	/// unreadable count is never one, so a policy is only ever deleted on a
	/// count that says nothing else holds it.
	fn release(attachments: u32) -> bool { attachments == 1 }

	/// One line per policy this launch's apps render: its name and a digest of
	/// its document, the deploy policy unconditioned since whether it carries
	/// the boundary conditions is an account read, not a declaration. Then one
	/// line per service-linked role they need ([`Self::SERVICE_LINKED_LINE`]).
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
		lines.extend(
			Self::service_linked_roles(apps).into_iter().map(|service| {
				format!("{} {service}", Self::SERVICE_LINKED_LINE)
			}),
		);
		lines.sort();
		lines.xok()
	}

	/// Buy an administrator session, converge the declared account
	/// ([`AwsAccount`]) and act inside it, converge the user, every boundary
	/// and policy, the key, and the user it replaces, and hand the deploy the
	/// session whatever
	/// `ask.run` says: it is bought already, and the key's policies may have
	/// changed a moment ago, which IAM takes a while to propagate.
	async fn elevate_with(
		&self,
		caller: &AsyncEntity,
		ask: ElevationAsk,
	) -> Result<CredentialElevation> {
		// rendered before the code is asked for, so a stack that cannot render
		// costs no session; a key not yet in the declared account renders
		// against its placeholder, since the launch may hold no credential
		let Some(apps) =
			Self::apps(caller, Self::moving_to(caller).await?).await?
		else {
			bevybail!(
				"this launch declares no stack keeping its state at AWS, so \
				there is no AWS deploy credential to elevate"
			);
		};
		let session = AdminSession::acquire(AdminSession::DEFAULT_SECS)
			.await
			.map_err(|err| {
			bevyhow!(
				"an elevated deploy asks for the six-digit code from the phone, \
				and {err}\n\nrun it in a terminal: `{}`",
				ask.relay
			)
		})?;
		let mut report = Vec::new();
		// the account first, since the credential lives inside it
		let management = AwsAdmin::new(session.sdk_vars());
		let (admin, home) = match AwsAccount::declared(caller).await? {
			Some((declared, region)) => {
				let (home, inside, lines) = declared
					.converge(
						&AwsManagement::new(management, region.clone()),
						&region,
					)
					.await?;
				report.extend(lines);
				(inside.into_admin(), Some(home))
			}
			None => (management, None),
		};
		let account = admin.account_id().await?;
		// lowered against the launch's state bucket, which is another
		// account's until the credential lives in the member: inside one it is
		// granted that account's own, as the plain deploys after resolve it
		let apps = match home {
			Some(_) => {
				Self::apps(caller, Some(&account)).await?.unwrap_or(apps)
			}
			None => apps,
		};
		let entry = DeployCredential::entry(caller).await?;
		let user = DeployCredential::name(&entry);
		report.push(admin.ensure_user(&user).await?);
		// what the account must hold that the key may never create
		for service in Self::service_linked_roles(&apps) {
			report.push(admin.ensure_service_linked_role(service).await?);
		}
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
		// exactly this entry's: a policy another entry's apps declare is
		// that entry's user's to hold
		let own = apps
			.iter()
			.map(|app| {
				format!(
					"arn:aws:iam::{account}:policy/{}",
					app.deploy.policy_name()
				)
			})
			.collect::<Vec<_>>();
		for arn in admin.attached_policies(&user).await? {
			if !own.contains(&arn.to_string()) {
				report.push(admin.release_policy(&user, &arn).await?);
			}
		}
		let grants = Self::grant_lines(&apps)?;
		let home = KeyHome {
			account: &account,
			member: home.as_ref(),
		};
		report.extend(
			Self::converge_key(
				caller, &admin, &entry, home, &apps, &grants, ask.roll,
			)
			.await?,
		);
		CredentialElevation {
			report,
			env: admin.vars().to_vec(),
			cleanup: None,
		}
		.xok()
	}

	/// One [`AppPolicies`] per app this launch declares, lowered from every
	/// stack of it at the launch's stage and `prod`, in app order, granting the
	/// state bucket of `account` (see [`Self::state_bucket`]). Renders in one
	/// world pass, so a stack that cannot render fails here rather than at the
	/// account. `None` when this launch deploys nothing at AWS: no stack, or
	/// state kept locally, which deploys as nobody.
	async fn apps(
		caller: &AsyncEntity,
		account: Option<&str>,
	) -> Result<Option<Vec<AppPolicies>>> {
		let Some(state_bucket) = Self::state_bucket(caller, account).await?
		else {
			return None.xok();
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
						deploy: AwsDeployPolicy::new(
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

	/// The state bucket the policies grant, `None` when state is kept locally:
	/// as seen from the AWS account `account` when the credential is to live
	/// there ([`S3Backend::bucket_in`]), else the one this launch resolves,
	/// which is the account of the credential it runs as.
	async fn state_bucket(
		caller: &AsyncEntity,
		account: Option<&str>,
	) -> Result<Option<SmolStr>> {
		let backend = caller
			.world()
			.with_resource::<Deployment, _>(|deployment| {
				deployment.backend().clone()
			})
			.await;
		match (backend, account) {
			(StackBackend::S3(s3), Some(account)) => {
				Some(s3.bucket_in(account))
			}
			(backend, _) => match backend.resolve().await? {
				ResolvedBackend::S3(s3) => Some(s3.bucket().clone()),
				ResolvedBackend::Local(_) => None,
			},
		}
		.xok()
	}

	/// The dry run's answer, needing no credential: the user and every policy
	/// document, pretty printed so it reads and pipes. The boundary conditions
	/// are absent, since whether they apply is an account read a dry run does
	/// not make.
	async fn describe_of(caller: &AsyncEntity) -> Result<String> {
		let declared = AwsAccount::declared(caller).await?;
		let Some(apps) =
			Self::apps(caller, Self::moving_to(caller).await?).await?
		else {
			return "aws: this launch declares no stack keeping its state at AWS\n"
				.to_string()
				.xok();
		};
		let mut out = format!(
			"user {}\n",
			DeployCredential::name(&DeployCredential::entry(caller).await?)
		);
		if let Some((account, region)) = declared {
			out.push_str(&format!("{}\n", account.describe(&region)?));
		}
		for service in Self::service_linked_roles(&apps) {
			out.push_str(&format!("service-linked role for {service}\n"));
		}
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

	/// Converge `entry`'s access key: mint one when the document holds none of
	/// its user's, or on `roll`, sealing the pair with `grants` before deleting
	/// what it replaces, then retire the user the record held a key of when it
	/// is another ([`Self::retiring`]); restate a current pair's records
	/// otherwise. The key id's record keeps the boundaries of stages this
	/// launch did not render ([`Self::with_kept`]) while it lives in the same
	/// account, and none once the credential moves into another, which holds
	/// none of them. One report line per act.
	async fn converge_key(
		caller: &AsyncEntity,
		admin: &AwsAdmin,
		entry: &str,
		home: KeyHome<'_>,
		apps: &[AppPolicies],
		grants: &[String],
		roll: bool,
	) -> Result<Vec<String>> {
		let user = DeployCredential::name(entry);
		let user = user.as_str();
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
		let same_home = held_id.as_ref().is_none_or(|secret| {
			secret
				.record
				.metadata
				.get(Self::ACCOUNT_NAME)
				.map(SmolStr::as_str)
				== home.member.map(|member| member.name.as_str())
		});
		let sealed = Self::with_kept(
			match same_home {
				true => DeployCredential::held_grants(held_id.as_ref()),
				false => Vec::new(),
			},
			apps,
			grants,
		);
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
				(held_id.as_ref(), Self::id_record(entry, &home, &sealed)),
				(held_secret.as_ref(), Self::secret_record(entry)),
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
			return vec![format!(
				"key {} of {user} is sealed in {}{}",
				held.unwrap_or_default(),
				handle.describe(),
				match resealed {
					true => ", its note, roll, grants and owner resealed",
					false => "",
				}
			)]
			.xok();
		}
		// the user the record held a key of, asked before the mint: a key
		// this user does not hold is another's, which this converge replaces
		let previous = match (same_home, held.as_ref()) {
			(true, Some(held)) if !existing.contains(held) => {
				admin.key_owner(held).await?
			}
			_ => None,
		};
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
			Self::id_record(entry, &home, &sealed),
		)?;
		document.set(
			&identity,
			&group,
			Self::KEY_SECRET,
			&secret,
			Self::secret_record(entry),
		)?;
		handle.write(&document).await?;
		// only now: the pair that replaces it is sealed and proven
		if let Some(held) =
			held.filter(|held| existing.iter().any(|key| key == held))
		{
			admin.delete_key(user, &held).await?;
			info!("deleted the replaced key {held} of {user}");
		}
		let mut report = vec![format!(
			"key {id} minted for {user} and sealed in {} (group `{group}`)",
			handle.describe()
		)];
		if let Some(retiring) = Self::retiring(previous.as_deref(), user) {
			report.extend(admin.retire_user(retiring).await?);
		}
		report.xok()
	}

	/// The user a converge of `user` retires once its pair is sealed: the one
	/// `previous`, the owner of the key the record held, names, unless that is
	/// `user` itself.
	fn retiring<'a>(previous: Option<&'a str>, user: &str) -> Option<&'a str> {
		previous.filter(|previous| *previous != user)
	}

	/// `entry`'s key id record: what it is, how it rolls, the entry owning it,
	/// the account it lives in, and the grants a plain deploy compares the
	/// declarations against.
	fn id_record(
		entry: &str,
		home: &KeyHome,
		grants: &[String],
	) -> SecretRecord {
		SecretRecord {
			role: Some(SecretRole::EnvVar),
			note: Some(
				format!(
					"aws deploy credential `{}`: the access key id of the \
					`{entry}` entry's own iam user; its deploy verbs and every \
					`aws s3 sync` read the pair from here, never from ~/.aws",
					DeployCredential::name(entry)
				)
				.into(),
			),
			roll: Some(SecretRoll::Elevated),
			metadata: [
				(
					DeployCredential::GRANTS,
					Some(SmolStr::from(grants.join("\n"))),
				),
				(DeployCredential::ENTRY, Some(SmolStr::from(entry))),
				(Self::ACCOUNT, Some(SmolStr::from(home.account))),
				(
					Self::ACCOUNT_NAME,
					home.member.map(|member| member.name.clone()),
				),
			]
			.into_iter()
			.filter_map(|(key, value)| {
				value.map(|value| (SmolStr::from(key), value))
			})
			.collect(),
			..default()
		}
	}

	/// `entry`'s secret half record.
	fn secret_record(entry: &str) -> SecretRecord {
		SecretRecord {
			role: Some(SecretRole::EnvVar),
			note: Some(
				format!(
					"aws deploy credential `{}`: the secret half of {}, \
					shown once at creation",
					DeployCredential::name(entry),
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
/// deploy pair, which is exactly the credential that cannot do this.
pub(crate) struct AwsAdmin(Vec<(SmolStr, SmolStr)>);

impl AwsAdmin {
	/// The session `vars` acts as.
	pub(crate) fn new(vars: Vec<(SmolStr, SmolStr)>) -> Self { Self(vars) }

	/// The session's three variables.
	pub(crate) fn vars(&self) -> &[(SmolStr, SmolStr)] { &self.0 }

	/// `process` as this session, answering its stdout.
	pub(crate) async fn run(&self, process: ChildProcess) -> Result<String> {
		self.wrap(process).run_async_stdout().await
	}

	/// A `--output text` answer, `None` when empty or the cli's `None`.
	pub(crate) fn text_option(out: String) -> Option<SmolStr> {
		Some(out.trim())
			.filter(|out| !out.is_empty() && *out != "None")
			.map(SmolStr::from)
	}

	/// Every object key in `bucket`, none when the bucket does not exist.
	pub(crate) async fn state_keys(
		&self,
		bucket: &str,
		region: &str,
	) -> Result<Vec<SmolStr>> {
		match self
			.run(aws_cli_ext::service("s3api", region, [
				"list-objects-v2",
				"--bucket",
				bucket,
				"--query",
				"Contents[].Key",
				"--output",
				"text",
			]))
			.await
		{
			Ok(out) => out
				.split_whitespace()
				.filter(|key| *key != "None")
				.map(SmolStr::from)
				.collect::<Vec<_>>()
				.xok(),
			Err(err) if err.to_string().contains("NoSuchBucket") => {
				Vec::new().xok()
			}
			Err(err) => Err(err),
		}
	}

	/// The tag the deploy user carries, saying what made it.
	const USER_TAG: &'static str = "Key=beet,Value=deploy-credential";
	/// Its value, what a retirement checks before deleting a user.
	const USER_TAG_VALUE: &'static str = "deploy-credential";

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
	pub(crate) async fn account_id(&self) -> Result<String> {
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
	pub(crate) async fn iam<'a>(
		&self,
		args: impl IntoIterator<Item = &'a str>,
	) -> Result<String> {
		self.run(aws_cli_ext::iam(args)).await
	}

	/// [`iam`](Self::iam) answering json, `None` when the entity does not
	/// exist: every converge step is get-or-create, and absence is the one
	/// failure that is not one.
	pub(crate) async fn iam_json<'a>(
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
			Self::USER_TAG,
		])
		.await?;
		format!("user {user} created").xok()
	}

	/// The account's service-linked role for `service`, created when absent.
	/// AWS names the role itself and keeps it under
	/// `/aws-service-role/<service>/`, so presence is a listing of that path.
	async fn ensure_service_linked_role(
		&self,
		service: &str,
	) -> Result<String> {
		let path = format!("/aws-service-role/{service}/");
		let held = self
			.iam([
				"list-roles",
				"--path-prefix",
				&path,
				"--query",
				"Roles[0].RoleName",
				"--output",
				"text",
			])
			.await?;
		if let Some(name) = Self::text_option(held) {
			return format!("service-linked role {name} exists").xok();
		}
		let name = self
			.iam([
				"create-service-linked-role",
				"--aws-service-name",
				service,
				"--query",
				"Role.RoleName",
				"--output",
				"text",
			])
			.await?;
		format!("service-linked role {} created for {service}", name.trim())
			.xok()
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

	/// The user holding the access key `id`, `None` when the account holds
	/// no such key.
	async fn key_owner(&self, id: &str) -> Result<Option<SmolStr>> {
		match self
			.iam([
				"get-access-key-last-used",
				"--access-key-id",
				id,
				"--query",
				"UserName",
				"--output",
				"text",
			])
			.await
		{
			Ok(out) => Self::text_option(out).xok(),
			Err(err)
				if err.to_string().contains("NoSuchEntity")
					|| err.to_string().contains("AccessDenied") =>
			{
				None.xok()
			}
			Err(err) => Err(err),
		}
	}

	/// Every managed policy attached to `user`, by arn.
	async fn attached_policies(&self, user: &str) -> Result<Vec<SmolStr>> {
		self.iam([
			"list-attached-user-policies",
			"--user-name",
			user,
			"--query",
			"AttachedPolicies[].PolicyArn",
			"--output",
			"json",
		])
		.await?
		.xmap(|body| serde_json::from_str::<Vec<SmolStr>>(&body))?
		.xok()
	}

	/// Detach the managed policy `arn` from `user`, deleting it when nothing
	/// else holds it ([`AwsDeployCredential::release`]), answering the report
	/// line.
	async fn release_policy(&self, user: &str, arn: &str) -> Result<String> {
		let attachments = self
			.iam([
				"get-policy",
				"--policy-arn",
				arn,
				"--query",
				"Policy.AttachmentCount",
				"--output",
				"text",
			])
			.await?
			.trim()
			.parse::<u32>()
			.unwrap_or(u32::MAX);
		self.iam([
			"detach-user-policy",
			"--user-name",
			user,
			"--policy-arn",
			arn,
		])
		.await?;
		let name = arn.rsplit('/').next().unwrap_or(arn);
		if !AwsDeployCredential::release(attachments) {
			return format!("policy {name} detached from {user}").xok();
		}
		// a managed policy is deleted only once its other versions are
		let versions = self
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
		for version in versions {
			self.iam([
				"delete-policy-version",
				"--policy-arn",
				arn,
				"--version-id",
				version.as_str(),
			])
			.await?;
		}
		self.iam(["delete-policy", "--policy-arn", arn]).await?;
		format!(
			"policy {name} detached from {user} and deleted, nothing else holding it"
		)
		.xok()
	}

	/// Retire the deploy user `user` a renamed credential replaced: every
	/// policy released ([`Self::release_policy`]), its keys deleted, then the
	/// user, one report line per act. A user this converge did not make
	/// (untagged) is left, and said so.
	async fn retire_user(&self, user: &str) -> Result<Vec<String>> {
		let tag = self
			.iam([
				"list-user-tags",
				"--user-name",
				user,
				"--query",
				"Tags[?Key==`beet`].Value",
				"--output",
				"text",
			])
			.await
			.ok()
			.and_then(Self::text_option);
		if tag.as_deref() != Some(Self::USER_TAG_VALUE) {
			return vec![format!(
				"user {user} held the replaced key but is no deploy user this \
				made, so it is left: delete it at the console if nothing needs \
				it"
			)]
			.xok();
		}
		let mut report = Vec::new();
		for arn in self.attached_policies(user).await? {
			report.push(self.release_policy(user, &arn).await?);
		}
		for key in self.access_keys(user).await? {
			self.delete_key(user, &key).await?;
			report.push(format!("key {key} of {user} deleted"));
		}
		self.iam(["delete-user", "--user-name", user]).await?;
		report
			.push(format!("user {user} retired, replaced by this entry's own"));
		report.xok()
	}
}

impl AppPolicies {
	/// This app's deploy policy, carrying the boundary conditions only when
	/// every principal its stacks render ALREADY wears the boundary.
	///
	/// The order of operations, enforced rather than remembered: see
	/// [`AwsDeployCredential`]. An app with no principals at the account yet is
	/// conditioned from birth, since its `CreateRole` carries the boundary.
	async fn deploy_policy(
		&self,
		admin: &AwsAdmin,
		account: &str,
	) -> Result<AwsDeployPolicy> {
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
		AwsDeployPolicy::converge_document(
			admin,
			account,
			&self.policy_name(),
			&self.to_json(),
		)
		.await
	}
}

impl AwsDeployPolicy {
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
	pub(crate) async fn converge_document(
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
	pub(crate) async fn attach(
		admin: &AwsAdmin,
		arn: &str,
		user: &str,
	) -> Result {
		if admin
			.attached_policies(user)
			.await?
			.iter()
			.any(|held| held == arn)
		{
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
	use super::AppPolicies;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// An elevated prod deploy renders prod alone, so it replaces the lines of
	/// the policies it lowers and keeps the dev boundary as it was sealed:
	/// dropping the dev line once left every later dev deploy refusing for
	/// `--elevated` over a boundary the account already held byte for byte.
	/// Another entry's line is dropped: its own user holds it.
	#[beet_core::test]
	fn keeps_the_grants_of_a_stage_it_did_not_render() {
		let apps = vec![AppPolicies {
			deploy: AwsDeployPolicy::new("beet-site", "state"),
			boundaries: BTreeMap::from([(
				SmolStr::new(BootstrapConfig::PROD_STAGE),
				RuntimeBoundary::new("beet-site", BootstrapConfig::PROD_STAGE),
			)]),
			principals: Vec::new(),
		}];
		let held = [
			"beet--deploy 11",
			"beet-site--deploy 22",
			"beet-site--dev--runtime-boundary 33",
			"beet-site--prod--runtime-boundary 44",
		]
		.map(SmolStr::from)
		.to_vec();
		AwsDeployCredential::with_kept(held, &apps, &[
			"beet-site--deploy 55".into(),
			"beet-site--prod--runtime-boundary 66".into(),
		])
		.xpect_eq(vec![
			"beet-site--deploy 55".to_string(),
			"beet-site--dev--runtime-boundary 33".into(),
			"beet-site--prod--runtime-boundary 66".into(),
		]);
		// only this entry's apps: a boundary of `beet-site-x` is not one
		AwsDeployCredential::keeps(
			&apps,
			"beet-site-x--dev--runtime-boundary 1",
		)
		.xpect_false();
		AwsDeployCredential::keeps(
			&apps,
			"beetmania--prod--runtime-boundary 1",
		)
		.xpect_false();
	}

	/// A converge's grants are exactly its entry's: a held line of another
	/// entry's app or a service-linked role nothing here needs is not kept,
	/// and a plain deploy notes it as the next elevated deploy's to drop.
	#[beet_core::test]
	fn seals_exactly_its_entrys_grants() {
		let apps = vec![AppPolicies {
			deploy: AwsDeployPolicy::new("beetmania", "state"),
			boundaries: BTreeMap::from([(
				SmolStr::new(BootstrapConfig::PROD_STAGE),
				RuntimeBoundary::new("beetmania", BootstrapConfig::PROD_STAGE),
			)]),
			principals: Vec::new(),
		}];
		let held = [
			"beet-site--deploy 22",
			"beetmania--deploy 11",
			"service-linked-role ops.apigateway.amazonaws.com",
		]
		.map(SmolStr::from)
		.to_vec();
		let grants = vec!["beetmania--deploy 33".to_string()];
		AwsDeployCredential::with_kept(held.clone(), &apps, &grants)
			.xpect_eq(grants.clone());
		let secret = Secret {
			name: "AWS_ACCESS_KEY_ID".into(),
			group: "default".into(),
			value: "AKIA".into(),
			record: SecretRecord {
				metadata: BTreeMap::from([(
					DeployCredential::GRANTS.into(),
					held.join("\n").into(),
				)]),
				..default()
			},
		};
		let rendered = |line: &str| !AwsDeployCredential::keeps(&apps, line);
		let status = CredentialStatus::compare(
			"aws",
			"AWS_ACCESS_KEY_ID",
			&grants,
			Some(&secret),
			Some(&rendered),
		);
		status.stale.len().xpect_eq(1);
		status
			.notes
			.join("\n")
			.xpect_contains("beet-site--deploy")
			.xpect_contains("service-linked-role");
	}

	/// A renamed credential retires the user its record held a key of, and
	/// only that user: the entry's own is never retired, and a policy
	/// detached from it is deleted only when the count says it was the one
	/// attachment.
	///
	/// REGRESSION: the repo's one `beet-deploy` user held every entry's
	/// policies; naming the credential per entry must leave no user behind.
	#[beet_core::test]
	fn retires_the_user_a_renamed_credential_replaced() {
		AwsDeployCredential::retiring(Some("beet-deploy"), "beet-site-deploy")
			.xpect_eq(Some("beet-deploy"));
		AwsDeployCredential::retiring(
			Some("beet-site-deploy"),
			"beet-site-deploy",
		)
		.xpect_none();
		AwsDeployCredential::retiring(None, "beet-site-deploy").xpect_none();
		// `beet-site--deploy` is attached to the new user too
		AwsDeployCredential::release(2).xpect_false();
		AwsDeployCredential::release(1).xpect_true();
		AwsDeployCredential::release(u32::MAX).xpect_false();
	}

	/// A service-linked role the stacks need is a grant line of its own, so a
	/// plain deploy into an account the converge has not prepared refuses.
	#[beet_core::test]
	fn records_the_service_linked_roles_the_stacks_need() {
		let app = |declared: &[&str]| AppPolicies {
			deploy: AwsDeployPolicy::test_lowered("my-site", declared),
			boundaries: BTreeMap::new(),
			principals: Vec::new(),
		};
		let line = "service-linked-role ops.apigateway.amazonaws.com";
		let domain = vec![app(&["aws_apigatewayv2_domain_name"])];
		AwsDeployCredential::grant_lines(&domain)
			.unwrap()
			.contains(&line.to_string())
			.xpect_true();
		// a role the entry no longer needs is dropped with the rest
		let bucket = vec![app(&[])];
		AwsDeployCredential::with_kept(vec![line.into()], &bucket, &[])
			.xpect_empty();
	}

	/// Declaring an account makes a key living anywhere else stale, the
	/// management account included, so the first elevated deploy moves it.
	#[beet_core::test]
	fn a_key_in_another_account_is_stale() {
		let held = |account: Option<&str>| Secret {
			name: "AWS_ACCESS_KEY_ID".into(),
			group: "default".into(),
			value: "AKIA".into(),
			record: SecretRecord {
				metadata: account
					.map(|account| {
						(SmolStr::new("account_name"), SmolStr::new(account))
					})
					.into_iter()
					.collect(),
				..default()
			},
		};
		AwsDeployCredential::elsewhere(&held(Some("beetmash")), "beetmash")
			.xpect_none();
		AwsDeployCredential::elsewhere(&held(None), "beetmash")
			.unwrap()
			.xpect_contains("the management account");
		AwsDeployCredential::elsewhere(&held(Some("beet")), "beetmash")
			.unwrap()
			.xpect_contains("lives in `beet`");
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
		AwsDeployCredential
			.protects(&change("aws_s3_bucket", "delete"), &prod)
			.xpect_some();
		AwsDeployCredential
			.protects(&change("aws_dynamodb_table", "replace"), &prod)
			.xpect_some();
		AwsDeployCredential
			.protects(&change("aws_s3_bucket", "update"), &prod)
			.xpect_none();
		AwsDeployCredential
			.protects(&change("aws_s3_bucket", "delete"), &dev)
			.xpect_none();
		AwsDeployCredential
			.protects(&change("aws_instance", "delete"), &prod)
			.xpect_none();
		AwsDeployCredential
			.protects(&change("aws_ebs_volume", "replace"), &prod)
			.xpect_some();
		AwsDeployCredential
			.protects(&change("aws_ebs_volume", "update"), &prod)
			.xpect_none();
		for kind in [
			"aws_s3_bucket_versioning",
			"aws_s3_bucket_lifecycle_configuration",
		] {
			AwsDeployCredential
				.protects(&change(kind, "create"), &prod)
				.xpect_some();
			AwsDeployCredential
				.protects(&change(kind, "update"), &prod)
				.xpect_some();
			AwsDeployCredential
				.protects(&change(kind, "update"), &dev)
				.xpect_none();
		}
	}
}

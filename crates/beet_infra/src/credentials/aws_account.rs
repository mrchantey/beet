//! `<AwsAccount/>`: the AWS account a repo deploys into, a member of the
//! operator's Organization, and its converge.

use super::aws_deploy_credential::AwsAdmin;
use crate::actions::aws_cli_ext;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;
use serde_json::json;

/// `<AwsAccount name="beetmash"/>`: the AWS account a repo deploys into, its
/// own member account of the operator's AWS Organization, declared at the
/// package root beside the [`AwsDeployCredential`] that deploys into it.
///
/// An account isolates a repo outright where a policy scoped by name only
/// narrows it: one repo's leaked deploy key reaches nothing of another's, and
/// each repo's SES standing and port 25 allowance are its own. The account is
/// global and its resources regional, so the region is the [`AwsRegion`] on
/// this declaration or an ancestor, which the state bucket and the Recycle
/// Bin rules land in.
///
/// ## What converges
///
/// Nobody creates it by hand: the elevated deploy that converges the AWS
/// deploy credential converges its account first, as the management account's
/// administrator ([`AdminSession`]), since the credential lives inside it.
/// Account level and outside tofu for the credential's own reason: the
/// account holds the state, so its state cannot hold the account.
///
/// 1. the Organization, enabled with every feature when absent;
/// 2. the service control policy keeping members in it
///    ([`MEMBER_GUARD`](Self::MEMBER_GUARD): no `LeaveOrganization`, no
///    `CloseAccount`, so closing stays the management account's act), on the
///    Organization's root, the policy type enabled first;
/// 3. `beet-agent`'s path to every member's read role, in the management
///    account;
/// 4. the member account, found by name, else created with a root email
///    plus-addressed from the sealed base ([`BASE_EMAIL`](Self::BASE_EMAIL))
///    and the creation instant;
/// 5. inside it, as [`MemberAccount::ADMIN_ROLE`]: one Recycle Bin rule per
///    resource type, region level and LOCKED, so a deleted snapshot or data
///    volume stays restorable for the declared days and a rule can only be
///    changed once an unlock's delay has passed; the read role
///    ([`MemberAccount::READ_ROLE`], `ViewOnlyAccess`) trusting `beet-agent`;
///    and the state bucket, versioned.
///
/// A rule that exists is left as it is, whatever its retention: a locked rule
/// changes only after an unlock and its delay, which is an operator's act at
/// the console, and the converge says when the two disagree.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct AwsAccount {
	/// The account's name in the Organization, found by it. Unset is the
	/// repo's name ([`DeployCredential::repo_name`]).
	pub name: Option<SmolStr>,
	/// The days a deleted EBS snapshot stays restorable, 1 to 365.
	pub snapshot_retention_days: u16,
	/// The days a deleted EBS volume stays restorable, 1 to 7.
	pub volume_retention_days: u16,
	/// The days an unlocked Recycle Bin rule waits before it can be changed
	/// or deleted, 7 to 30: the window to notice an unlock nobody meant.
	pub unlock_delay_days: u16,
}

impl Default for AwsAccount {
	fn default() -> Self {
		Self {
			name: None,
			snapshot_retention_days: 14,
			volume_retention_days: 7,
			unlock_delay_days: 7,
		}
	}
}

impl DeployAccountProvider for AwsAccount {
	fn id(&self) -> &'static str { "aws" }

	fn name(&self) -> SmolStr { self.resolved_name().unwrap_or_default() }

	fn close(&self, caller: AsyncEntity) -> SendBoxedFuture<Result<String>> {
		Box::pin(async move { Self::close_declared(&caller).await })
	}
}

impl AwsAccount {
	/// The record of the operator's global document holding the address every
	/// member's root email is plus-addressed from. Sealed rather than
	/// declared, since the repos that declare accounts are public.
	pub const BASE_EMAIL: &'static str = "AWS_ACCOUNT_BASE_EMAIL";
	/// The service control policy on the Organization's root keeping every
	/// member in it.
	pub const MEMBER_GUARD: &'static str = "beet-member-guard";
	/// The management account's policy letting `beet-agent` find a member
	/// and assume its read role.
	pub const AGENT_READ_POLICY: &'static str = "beet-agent--member-read";
	/// The Recycle Bin resource types a rule is converged for, each with the
	/// field its retention is declared by.
	const RULES: [&'static str; 2] = ["EBS_SNAPSHOT", "EBS_VOLUME"];
	/// The description the converge's own rules carry, which is how it finds
	/// them again: a rule's tags are not in its listing.
	const RULE_DESCRIPTION: &'static str =
		"beet: deleted resources stay restorable";
	/// How many times a new account's creation is polled, five seconds apart.
	const CREATE_POLLS: usize = 60;

	/// The name, the repo's when unset.
	pub(crate) fn resolved_name(&self) -> Result<SmolStr> {
		match &self.name {
			Some(name) => name.clone().xok(),
			None => SmolStr::from(DeployCredential::repo_name()?).xok(),
		}
	}

	/// Refuse a retention or delay outside what the Recycle Bin accepts,
	/// before anybody is asked for a code.
	fn check(&self) -> Result {
		for (field, value, range) in [
			(
				"snapshot_retention_days",
				self.snapshot_retention_days,
				1..=365,
			),
			("volume_retention_days", self.volume_retention_days, 1..=7),
			("unlock_delay_days", self.unlock_delay_days, 7..=30),
		] {
			if !range.contains(&value) {
				bevybail!(
					"`<AwsAccount {field}={value}/>` is outside what the \
					Recycle Bin accepts: {} to {} days",
					range.start(),
					range.end()
				);
			}
		}
		OK
	}

	/// The days a rule of `resource_type` retains for.
	fn retention(&self, resource_type: &str) -> u16 {
		match resource_type {
			"EBS_VOLUME" => self.volume_retention_days,
			_ => self.snapshot_retention_days,
		}
	}

	/// The root email of an account created at `millis`: the base address
	/// plus-addressed with the account's name and the instant, ie
	/// `pete+aws-beetmash-1791530298828@example.com`. Unique per creation,
	/// since AWS refuses a closed account's email for as long as it holds the
	/// closed account, and not the account id, which AWS assigns in answer to
	/// the call that needs the email.
	fn root_email(base: &str, name: &str, millis: u128) -> Result<String> {
		let Some((local, domain)) =
			base.trim().split_once('@').filter(|(local, domain)| {
				!local.is_empty()
					&& domain.contains('.')
					&& !local.contains('+')
			})
		else {
			bevybail!(
				"the sealed `{}` is not a plain address (`local@domain`, no \
				`+`), so no root email can be plus-addressed from it",
				Self::BASE_EMAIL
			);
		};
		format!("{local}+aws-{name}-{millis}@{domain}").xok()
	}

	/// The declared account and the region it lands in, `None` when this
	/// launch declares no `<AwsAccount/>`.
	pub(crate) async fn declared(
		caller: &AsyncEntity,
	) -> Result<Option<(Self, SmolStr)>> {
		caller
			.with_state::<(Query<(Entity, &AwsAccount)>, AncestorQuery<&AwsRegion>), _>(
				|_, (accounts, regions)| {
					let mut found = accounts.iter();
					let Some((entity, account)) = found.next() else {
						return None.xok();
					};
					if found.next().is_some() {
						bevybail!(
							"this launch declares two `<AwsAccount/>`s, and a \
							repo deploys into one: keep one declaration"
						);
					}
					let region = regions.get(entity).map_err(|_| {
						bevyhow!(
							"`<AwsAccount/>` needs an `AwsRegion` on it or an \
							ancestor: the state bucket and the Recycle Bin rules \
							land there"
						)
					})?;
					Some((account.clone(), region.0.clone())).xok()
				},
			)
			.await?
	}

	/// Converge the account, see the type docs, returning the member, the
	/// calls now acting inside it, and one line per thing converged.
	pub(crate) async fn converge<C: ManagementCalls>(
		&self,
		calls: &C,
		region: &str,
	) -> Result<(MemberAccount, C::Member, Vec<String>)> {
		self.check()?;
		let name = self.resolved_name()?;
		let mut report = Vec::new();
		// 1. the Organization
		if !calls.organization_exists().await? {
			calls.create_organization().await?;
			report.push("organization created, every feature".to_string());
		}
		// 2. the guard on the root, its policy type first
		let (root, enabled) = calls.root().await?;
		if !enabled {
			calls.enable_policies(&root).await?;
			report.push(format!("service control policies enabled on {root}"));
		}
		let guard = match calls.find_policy(Self::MEMBER_GUARD).await? {
			Some(guard) => guard,
			None => {
				let guard = calls
					.create_policy(
						Self::MEMBER_GUARD,
						&Self::member_guard().to_string(),
					)
					.await?;
				report.push(format!("policy {} created", Self::MEMBER_GUARD));
				guard
			}
		};
		if !calls.attached_policies(&root).await?.contains(&guard) {
			calls.attach_policy(&guard, &root).await?;
			report.push(format!(
				"policy {} attached to {root}",
				Self::MEMBER_GUARD
			));
		}
		// 3. beet-agent's path to every member's read role
		let management = calls.management_id().await?;
		report.push(
			calls
				.ensure_agent_read(
					Self::AGENT_READ_POLICY,
					&Self::agent_read().to_string(),
				)
				.await?,
		);
		// 4. the member, found or created
		let id = match calls.find_account(&name).await? {
			Some(id) => {
				report.push(format!("account {name} ({id}) exists"));
				id
			}
			None => {
				let email = Self::root_email(
					&calls.base_email().await?,
					&name,
					calls.now_millis(),
				)?;
				let request = calls.create_account(&name, &email).await?;
				let id = Self::await_created(calls, &request).await?;
				report.push(format!("account {name} ({id}) created"));
				id
			}
		};
		let member = MemberAccount { name, id };
		// 5. inside it
		let inside = calls.enter(&member).await?;
		for resource_type in Self::RULES {
			let days = self.retention(resource_type);
			match inside
				.find_rule(resource_type, Self::RULE_DESCRIPTION)
				.await?
			{
				Some(held) if held == days => report.push(format!(
					"recycle bin keeps {resource_type} {days} days, locked"
				)),
				Some(held) => report.push(format!(
					"recycle bin keeps {resource_type} {held} days, not the \
					declared {days}: the rule is locked, so changing it is an \
					unlock and its {}-day delay, by hand",
					self.unlock_delay_days
				)),
				None => {
					inside
						.create_rule(
							resource_type,
							Self::RULE_DESCRIPTION,
							days,
							self.unlock_delay_days,
						)
						.await?;
					report.push(format!(
						"recycle bin rule for {resource_type} created, {days} \
						days, locked with a {}-day unlock delay",
						self.unlock_delay_days
					));
				}
			}
		}
		if !inside.role_exists(MemberAccount::READ_ROLE).await? {
			inside
				.create_read_role(
					MemberAccount::READ_ROLE,
					&format!(
						"arn:aws:iam::{management}:user/{}",
						AgentIdentity::USER
					),
				)
				.await?;
			report.push(format!("role {} created", MemberAccount::READ_ROLE));
		}
		let bucket = S3Backend::default_bucket(&member.id);
		if !inside.bucket_exists(&bucket).await? {
			inside.create_state_bucket(&bucket, region).await?;
			report.push(format!("state bucket {bucket} created in {region}"));
		}
		(member, inside, report).xok()
	}

	/// Poll a creation request until it settles.
	async fn await_created<C: ManagementCalls>(
		calls: &C,
		request: &str,
	) -> Result<SmolStr> {
		for _ in 0..Self::CREATE_POLLS {
			if let Some(id) = calls.account_created(request).await? {
				return id.xok();
			}
			calls.wait().await;
		}
		bevybail!(
			"account creation `{request}` had not finished after {} polls: \
			rerun the elevated deploy, which finds the account by name once it \
			exists",
			Self::CREATE_POLLS
		)
	}

	/// The service control policy keeping every member in the Organization.
	fn member_guard() -> Value {
		json!({
			"Version": "2012-10-17",
			"Statement": [{
				"Sid": "MembersStay",
				"Effect": "Deny",
				"Action": ["organizations:LeaveOrganization", "account:CloseAccount"],
				"Resource": "*",
			}],
		})
	}

	/// `beet-agent`'s path to a member: finding it by name, and assuming its
	/// read role. Granted here rather than left to `ViewOnlyAccess`'s
	/// contents, so the read path never depends on what AWS puts in it.
	fn agent_read() -> Value {
		json!({
			"Version": "2012-10-17",
			"Statement": [
				{
					"Sid": "FindMembers",
					"Effect": "Allow",
					"Action": "organizations:ListAccounts",
					"Resource": "*",
				},
				{
					"Sid": "AssumeMemberRead",
					"Effect": "Allow",
					"Action": "sts:AssumeRole",
					"Resource": format!("arn:aws:iam::*:role/{}", MemberAccount::READ_ROLE),
				},
			],
		})
	}

	/// The dry run's line for the account, needing no credential.
	pub(crate) fn describe(&self, region: &str) -> Result<String> {
		format!(
			"account {} (a member of the Organization, in {region}): \
			deleted EBS snapshots kept {} days and volumes {}, the rules locked \
			with a {}-day unlock delay, the state bucket `{}<id>`",
			self.resolved_name()?,
			self.snapshot_retention_days,
			self.volume_retention_days,
			self.unlock_delay_days,
			S3Backend::DEFAULT_PREFIX
		)
		.xok()
	}

	/// Close the declared account: the administrator session, the member
	/// found, its state bucket read for any stack's state, then
	/// `organizations close-account` as the management account.
	async fn close_declared(caller: &AsyncEntity) -> Result<String> {
		let Some((account, region)) = Self::declared(caller).await? else {
			bevybail!("this launch declares no `<AwsAccount/>` to close");
		};
		let name = account.resolved_name()?;
		let session = AdminSession::acquire(AdminSession::DEFAULT_SECS).await?;
		let management = AwsAdmin::new(session.sdk_vars());
		let member = MemberAccount::find(management.vars(), &name).await?;
		let inside = AwsAdmin::new(
			member
				.assume(management.vars(), MemberAccount::ADMIN_ROLE)
				.await?
				.sdk_vars(),
		);
		let keys = inside
			.state_keys(&S3Backend::default_bucket(&member.id), &region)
			.await?;
		Self::refuse_live_state(&member, &keys)?;
		management
			.run(aws_cli_ext::organizations([
				"close-account",
				"--account-id",
				member.id.as_str(),
			]))
			.await?;
		format!(
			"closed `{}` ({}): suspended and reopenable for 90 days with its \
			resources kept, then deleted for good; the account id and its root \
			email are never usable again",
			member.name, member.id
		)
		.xok()
	}

	/// Refuse to close `member` while its state bucket holds any stack's
	/// state: `keys` are the bucket's objects.
	fn refuse_live_state(member: &MemberAccount, keys: &[SmolStr]) -> Result {
		let live = keys
			.iter()
			.filter(|key| key.contains("tfstate"))
			.map(|key| format!("`{key}`"))
			.collect::<Vec<_>>();
		if live.is_empty() {
			return OK;
		}
		bevybail!(
			"`{}` ({}) still holds the state of {}: destroy each stack first \
			(`<stack>/destroy --elevated`), since a closed account takes its \
			resources with it after 90 days and the state is what says they \
			exist",
			member.name,
			member.id,
			live.join(", ")
		)
	}
}

/// What an account converge asks of the management account, as its
/// administrator. A trait so the converge's order is testable against a fake;
/// [`AwsManagement`] is the cli.
pub(crate) trait ManagementCalls {
	/// The calls inside a member, once entered.
	type Member: MemberCalls;
	async fn organization_exists(&self) -> Result<bool>;
	async fn create_organization(&self) -> Result;
	/// The root's id, and whether service control policies are enabled on it.
	async fn root(&self) -> Result<(SmolStr, bool)>;
	async fn enable_policies(&self, root: &str) -> Result;
	/// The id of the service control policy named `name`.
	async fn find_policy(&self, name: &str) -> Result<Option<SmolStr>>;
	async fn create_policy(
		&self,
		name: &str,
		document: &str,
	) -> Result<SmolStr>;
	/// The ids of the service control policies attached to `target`.
	async fn attached_policies(&self, target: &str) -> Result<Vec<SmolStr>>;
	async fn attach_policy(&self, policy: &str, target: &str) -> Result;
	/// The management account's own id.
	async fn management_id(&self) -> Result<SmolStr>;
	/// Converge `beet-agent`'s managed policy `name` to `document`, attached.
	async fn ensure_agent_read(
		&self,
		name: &str,
		document: &str,
	) -> Result<String>;
	/// The active account named `name`.
	async fn find_account(&self, name: &str) -> Result<Option<SmolStr>>;
	/// The sealed base address a root email is plus-addressed from.
	async fn base_email(&self) -> Result<SmolStr>;
	/// The current instant in unix milliseconds.
	fn now_millis(&self) -> u128;
	/// Start creating an account, answering the request id.
	async fn create_account(&self, name: &str, email: &str) -> Result<SmolStr>;
	/// The account a creation request made, `None` while it is in progress.
	async fn account_created(&self, request: &str) -> Result<Option<SmolStr>>;
	/// Pause between polls.
	async fn wait(&self);
	/// Act inside `member` as its administrator role from here on.
	async fn enter(&self, member: &MemberAccount) -> Result<Self::Member>;
}

/// What an account converge asks inside the member, see [`ManagementCalls`].
pub(crate) trait MemberCalls {
	/// The retention days of the region-level rule for `resource_type`
	/// carrying `description`.
	async fn find_rule(
		&self,
		resource_type: &str,
		description: &str,
	) -> Result<Option<u16>>;
	/// A region-level rule for `resource_type`, created locked.
	async fn create_rule(
		&self,
		resource_type: &str,
		description: &str,
		days: u16,
		unlock_delay_days: u16,
	) -> Result;
	async fn role_exists(&self, name: &str) -> Result<bool>;
	/// The read role `name`, trusting `principal`, holding `ViewOnlyAccess`.
	async fn create_read_role(&self, name: &str, principal: &str) -> Result;
	async fn bucket_exists(&self, bucket: &str) -> Result<bool>;
	/// The state bucket, in `region`, versioned and private.
	async fn create_state_bucket(&self, bucket: &str, region: &str) -> Result;
}

/// The management account's administrator session making the calls through
/// the aws cli, see [`ManagementCalls`].
pub(crate) struct AwsManagement {
	admin: AwsAdmin,
	region: SmolStr,
}

impl AwsManagement {
	pub(crate) fn new(admin: AwsAdmin, region: impl Into<SmolStr>) -> Self {
		Self {
			admin,
			region: region.into(),
		}
	}

	/// An `aws organizations` call answering text.
	async fn organizations<'a>(
		&self,
		args: impl IntoIterator<Item = &'a str>,
	) -> Result<String> {
		self.admin.run(aws_cli_ext::organizations(args)).await
	}
}

impl ManagementCalls for AwsManagement {
	type Member = AwsMember;

	async fn organization_exists(&self) -> Result<bool> {
		match self.organizations(["describe-organization"]).await {
			Ok(_) => true.xok(),
			Err(err)
				if err.to_string().contains("AWSOrganizationsNotInUse") =>
			{
				false.xok()
			}
			Err(err) => Err(err),
		}
	}

	async fn create_organization(&self) -> Result {
		self.organizations(["create-organization", "--feature-set", "ALL"])
			.await
			.map(|_| ())
	}

	async fn root(&self) -> Result<(SmolStr, bool)> {
		let out = self
			.organizations([
				"list-roots",
				"--query",
				"Roots[0].[Id, PolicyTypes[?Type=='SERVICE_CONTROL_POLICY'].Status | [0]]",
				"--output",
				"text",
			])
			.await?;
		let mut fields = out.split_whitespace();
		let root = fields
			.next()
			.ok_or_else(|| bevyhow!("the Organization answered no root"))?;
		(root.into(), fields.next() == Some("ENABLED")).xok()
	}

	async fn enable_policies(&self, root: &str) -> Result {
		self.organizations([
			"enable-policy-type",
			"--root-id",
			root,
			"--policy-type",
			"SERVICE_CONTROL_POLICY",
		])
		.await
		.map(|_| ())
	}

	async fn find_policy(&self, name: &str) -> Result<Option<SmolStr>> {
		let query = format!("Policies[?Name=='{name}'].Id | [0]");
		self.organizations([
			"list-policies",
			"--filter",
			"SERVICE_CONTROL_POLICY",
			"--query",
			query.as_str(),
			"--output",
			"text",
		])
		.await
		.map(AwsAdmin::text_option)
	}

	async fn create_policy(
		&self,
		name: &str,
		document: &str,
	) -> Result<SmolStr> {
		self.organizations([
			"create-policy",
			"--name",
			name,
			"--type",
			"SERVICE_CONTROL_POLICY",
			"--description",
			"members stay in the Organization; closing is the management account's act",
			"--content",
			document,
			"--query",
			"Policy.PolicySummary.Id",
			"--output",
			"text",
		])
		.await
		.map(|out| out.trim().into())
	}

	async fn attached_policies(&self, target: &str) -> Result<Vec<SmolStr>> {
		self.organizations([
			"list-policies-for-target",
			"--target-id",
			target,
			"--filter",
			"SERVICE_CONTROL_POLICY",
			"--query",
			"Policies[].Id",
			"--output",
			"text",
		])
		.await
		.map(|out| out.split_whitespace().map(SmolStr::from).collect())
	}

	async fn attach_policy(&self, policy: &str, target: &str) -> Result {
		// a policy type just enabled is `PENDING_ENABLE` for a moment, and an
		// attach then answers `PolicyTypeNotEnabled`
		let mut last = None;
		for _ in 0..10 {
			match self
				.organizations([
					"attach-policy",
					"--policy-id",
					policy,
					"--target-id",
					target,
				])
				.await
			{
				Ok(_) => return OK,
				Err(err)
					if err.to_string().contains("PolicyTypeNotEnabled") =>
				{
					last = Some(err);
					time_ext::sleep_secs(3).await;
				}
				Err(err) => return Err(err),
			}
		}
		Err(last.unwrap_or_else(|| bevyhow!("attach-policy never succeeded")))
	}

	async fn management_id(&self) -> Result<SmolStr> {
		self.admin.account_id().await.map(SmolStr::from)
	}

	async fn ensure_agent_read(
		&self,
		name: &str,
		document: &str,
	) -> Result<String> {
		let account = self.admin.account_id().await?;
		let document = serde_json::from_str::<Value>(document)?;
		let report = AwsDeployPolicy::converge_document(
			&self.admin,
			&account,
			name,
			&document,
		)
		.await?;
		AwsDeployPolicy::attach(
			&self.admin,
			&format!("arn:aws:iam::{account}:policy/{name}"),
			AgentIdentity::USER,
		)
		.await?;
		report.xok()
	}

	async fn find_account(&self, name: &str) -> Result<Option<SmolStr>> {
		MemberAccount::lookup(self.admin.vars(), name)
			.await
			.map(|found| found.map(|member| member.id))
	}

	async fn base_email(&self) -> Result<SmolStr> {
		AgentIdentity::record(AwsAccount::BASE_EMAIL)
			.await?
			.ok_or_else(|| {
				bevyhow!(
					"no `{0}` in the global document, so a new account has no \
					root email: seal the base address once with `beet \
					secrets/set {0} --document=~/.config/beet/secrets.toml \
					--value=<you@example.com> --note=\"the base address every \
					aws member account's root email is plus-addressed from\"`",
					AwsAccount::BASE_EMAIL
				)
			})
	}

	fn now_millis(&self) -> u128 { Timestamp::now().millis() as u128 }

	async fn create_account(&self, name: &str, email: &str) -> Result<SmolStr> {
		self.organizations([
			"create-account",
			"--email",
			email,
			"--account-name",
			name,
			"--role-name",
			MemberAccount::ADMIN_ROLE,
			"--query",
			"CreateAccountStatus.Id",
			"--output",
			"text",
		])
		.await
		.map(|out| out.trim().into())
	}

	async fn account_created(&self, request: &str) -> Result<Option<SmolStr>> {
		let out = self
			.organizations([
				"describe-create-account-status",
				"--create-account-request-id",
				request,
				"--query",
				"CreateAccountStatus.[State, AccountId, FailureReason]",
				"--output",
				"text",
			])
			.await?;
		let fields = out.split_whitespace().collect::<Vec<_>>();
		match fields.as_slice() {
			["SUCCEEDED", id, ..] => Some(SmolStr::from(*id)).xok(),
			["IN_PROGRESS", ..] => None.xok(),
			[_, _, reason, ..] => {
				bevybail!("creating the account failed: {reason}")
			}
			_ => bevybail!("creating the account answered `{}`", out.trim()),
		}
	}

	async fn wait(&self) { time_ext::sleep_secs(5).await; }

	async fn enter(&self, member: &MemberAccount) -> Result<AwsMember> {
		// a new account's role is assumable a few seconds after it exists
		let mut last = None;
		for _ in 0..12 {
			match member
				.assume(self.admin.vars(), MemberAccount::ADMIN_ROLE)
				.await
			{
				Ok(session) => {
					return AwsMember {
						admin: AwsAdmin::new(session.sdk_vars()),
						region: self.region.clone(),
					}
					.xok();
				}
				Err(err) => {
					last = Some(err);
					time_ext::sleep_secs(5).await;
				}
			}
		}
		Err(last.unwrap_or_else(|| bevyhow!("the member was never entered")))
	}
}

/// The administrator role inside a member making the calls through the aws
/// cli, see [`MemberCalls`].
pub(crate) struct AwsMember {
	admin: AwsAdmin,
	region: SmolStr,
}

impl AwsMember {
	/// The session inside the member, for the deploy credential's converge.
	pub(crate) fn into_admin(self) -> AwsAdmin { self.admin }
}

impl MemberCalls for AwsMember {
	async fn find_rule(
		&self,
		resource_type: &str,
		description: &str,
	) -> Result<Option<u16>> {
		let query = format!(
			"Rules[?Description=='{description}'].RetentionPeriod.RetentionPeriodValue | [0]"
		);
		let out = self
			.admin
			.run(aws_cli_ext::service("rbin", &self.region, [
				"list-rules",
				"--resource-type",
				resource_type,
				"--query",
				query.as_str(),
				"--output",
				"text",
			]))
			.await?;
		AwsAdmin::text_option(out)
			.map(|days| days.parse::<u16>())
			.transpose()?
			.xok()
	}

	async fn create_rule(
		&self,
		resource_type: &str,
		description: &str,
		days: u16,
		unlock_delay_days: u16,
	) -> Result {
		let retention = json!({
			"RetentionPeriodValue": days,
			"RetentionPeriodUnit": "DAYS",
		})
		.to_string();
		let lock = json!({
			"UnlockDelay": {
				"UnlockDelayValue": unlock_delay_days,
				"UnlockDelayUnit": "DAYS",
			},
		})
		.to_string();
		self.admin
			.run(aws_cli_ext::service("rbin", &self.region, [
				"create-rule",
				"--resource-type",
				resource_type,
				"--description",
				description,
				"--retention-period",
				retention.as_str(),
				"--lock-configuration",
				lock.as_str(),
			]))
			.await
			.map(|_| ())
	}

	async fn role_exists(&self, name: &str) -> Result<bool> {
		self.admin
			.iam_json(["get-role", "--role-name", name])
			.await
			.map(|found| found.is_some())
	}

	async fn create_read_role(&self, name: &str, principal: &str) -> Result {
		let trust = json!({
			"Version": "2012-10-17",
			"Statement": [{
				"Effect": "Allow",
				"Principal": { "AWS": principal },
				"Action": "sts:AssumeRole",
			}],
		})
		.to_string();
		self.admin
			.iam([
				"create-role",
				"--role-name",
				name,
				"--assume-role-policy-document",
				trust.as_str(),
				"--tags",
				"Key=beet,Value=agent-read",
			])
			.await?;
		self.admin
			.iam([
				"attach-role-policy",
				"--role-name",
				name,
				"--policy-arn",
				"arn:aws:iam::aws:policy/job-function/ViewOnlyAccess",
			])
			.await
			.map(|_| ())
	}

	async fn bucket_exists(&self, bucket: &str) -> Result<bool> {
		match self
			.admin
			.run(aws_cli_ext::service("s3api", &self.region, [
				"head-bucket",
				"--bucket",
				bucket,
			]))
			.await
		{
			Ok(_) => true.xok(),
			Err(err)
				if ["404", "Not Found", "NoSuchBucket"]
					.iter()
					.any(|marker| err.to_string().contains(marker)) =>
			{
				false.xok()
			}
			Err(err) => Err(err),
		}
	}

	async fn create_state_bucket(&self, bucket: &str, region: &str) -> Result {
		let location = format!("LocationConstraint={region}");
		let mut args = vec!["create-bucket", "--bucket", bucket];
		// us-east-1 is the one region that refuses to be named as a constraint
		if region != crate::bindings::aws::region::US_EAST_1 {
			args.extend(["--create-bucket-configuration", location.as_str()]);
		}
		self.admin
			.run(aws_cli_ext::service("s3api", region, args))
			.await?;
		// every apply overwrites the state in place, so a version is its only
		// undo, as for a bucket a first deploy creates
		self.admin
			.run(aws_cli_ext::service("s3api", region, [
				"put-bucket-versioning",
				"--bucket",
				bucket,
				"--versioning-configuration",
				"Status=Enabled",
			]))
			.await?;
		self.admin
			.run(aws_cli_ext::service("s3api", region, [
				"put-public-access-block",
				"--bucket",
				bucket,
				"--public-access-block-configuration",
				"BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true",
			]))
			.await
			.map(|_| ())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// The calls a converge made, in order, against an account where nothing
	/// exists yet (`fresh`) or where everything already does.
	#[derive(Clone, Copy)]
	struct Fake {
		fresh: bool,
		log: Store<Vec<String>>,
	}

	impl Fake {
		fn new(fresh: bool) -> Self {
			Self {
				fresh,
				log: Store::default(),
			}
		}

		fn call(&self, line: impl Into<String>) {
			let mut log = self.log.get();
			log.push(line.into());
			self.log.set(log);
		}
	}

	impl ManagementCalls for Fake {
		type Member = Fake;
		async fn organization_exists(&self) -> Result<bool> {
			(!self.fresh).xok()
		}
		async fn create_organization(&self) -> Result {
			self.call("create-organization");
			OK
		}
		async fn root(&self) -> Result<(SmolStr, bool)> {
			(SmolStr::new("r-root"), !self.fresh).xok()
		}
		async fn enable_policies(&self, root: &str) -> Result {
			self.call(format!("enable-policy-type {root}"));
			OK
		}
		async fn find_policy(&self, _name: &str) -> Result<Option<SmolStr>> {
			(!self.fresh).then(|| SmolStr::new("p-guard")).xok()
		}
		async fn create_policy(
			&self,
			name: &str,
			document: &str,
		) -> Result<SmolStr> {
			self.call(format!("create-policy {name}"));
			document.xpect_contains("organizations:LeaveOrganization");
			SmolStr::new("p-guard").xok()
		}
		async fn attached_policies(
			&self,
			_target: &str,
		) -> Result<Vec<SmolStr>> {
			match self.fresh {
				true => vec![SmolStr::new("p-full-access")],
				false => {
					vec![SmolStr::new("p-full-access"), SmolStr::new("p-guard")]
				}
			}
			.xok()
		}
		async fn attach_policy(&self, policy: &str, target: &str) -> Result {
			self.call(format!("attach-policy {policy} {target}"));
			OK
		}
		async fn management_id(&self) -> Result<SmolStr> {
			SmolStr::new("111").xok()
		}
		async fn ensure_agent_read(
			&self,
			name: &str,
			_document: &str,
		) -> Result<String> {
			self.call(format!("agent-read {name}"));
			format!("policy {name} unchanged").xok()
		}
		async fn find_account(&self, _name: &str) -> Result<Option<SmolStr>> {
			(!self.fresh).then(|| SmolStr::new("222")).xok()
		}
		async fn base_email(&self) -> Result<SmolStr> {
			SmolStr::new("pete@example.com").xok()
		}
		fn now_millis(&self) -> u128 { 1791530298828 }
		async fn create_account(
			&self,
			name: &str,
			email: &str,
		) -> Result<SmolStr> {
			self.call(format!("create-account {name} {email}"));
			SmolStr::new("car-1").xok()
		}
		async fn account_created(
			&self,
			request: &str,
		) -> Result<Option<SmolStr>> {
			self.call(format!("poll {request}"));
			Some(SmolStr::new("222")).xok()
		}
		async fn wait(&self) {}
		async fn enter(&self, member: &MemberAccount) -> Result<Fake> {
			self.call(format!("enter {}", member.id));
			(*self).xok()
		}
	}

	impl MemberCalls for Fake {
		async fn find_rule(
			&self,
			_resource_type: &str,
			_description: &str,
		) -> Result<Option<u16>> {
			(!self.fresh).then_some(30).xok()
		}
		async fn create_rule(
			&self,
			resource_type: &str,
			_description: &str,
			days: u16,
			unlock_delay_days: u16,
		) -> Result {
			self.call(format!(
				"create-rule {resource_type} {days} {unlock_delay_days}"
			));
			OK
		}
		async fn role_exists(&self, _name: &str) -> Result<bool> {
			(!self.fresh).xok()
		}
		async fn create_read_role(
			&self,
			name: &str,
			principal: &str,
		) -> Result {
			self.call(format!("create-role {name} {principal}"));
			OK
		}
		async fn bucket_exists(&self, _bucket: &str) -> Result<bool> {
			(!self.fresh).xok()
		}
		async fn create_state_bucket(
			&self,
			bucket: &str,
			region: &str,
		) -> Result {
			self.call(format!("create-bucket {bucket} {region}"));
			OK
		}
	}

	fn beetmash() -> AwsAccount {
		AwsAccount {
			name: Some("beetmash".into()),
			..default()
		}
	}

	/// From nothing, the converge goes in the only order that works: the
	/// Organization, its guard enabled then attached, the agent's read path,
	/// the account created and polled, and only then anything inside it.
	#[beet_core::test]
	async fn converges_in_order() {
		let fake = Fake::new(true);
		let (member, _, report) =
			beetmash().converge(&fake, "ap-southeast-2").await.unwrap();
		member.id.as_str().xpect_eq("222");
		fake.log.get().xpect_eq(vec![
			"create-organization".to_string(),
			"enable-policy-type r-root".into(),
			"create-policy beet-member-guard".into(),
			"attach-policy p-guard r-root".into(),
			"agent-read beet-agent--member-read".into(),
			"create-account beetmash pete+aws-beetmash-1791530298828@example.com".into(),
			"poll car-1".into(),
			"enter 222".into(),
			"create-rule EBS_SNAPSHOT 14 7".into(),
			"create-rule EBS_VOLUME 7 7".into(),
			"create-role beet-agent-read arn:aws:iam::111:user/beet-agent".into(),
			"create-bucket beet-state-222 ap-southeast-2".into(),
		]);
		report.len().xpect_greater_than(8);
	}

	/// Converged already, it creates nothing, and a locked rule holding other
	/// days than declared is reported rather than touched.
	#[beet_core::test]
	async fn a_converged_account_changes_nothing() {
		let fake = Fake::new(false);
		let (_, _, report) =
			beetmash().converge(&fake, "ap-southeast-2").await.unwrap();
		fake.log.get().xpect_eq(vec![
			"agent-read beet-agent--member-read".to_string(),
			"enter 222".into(),
		]);
		report
			.join("\n")
			.xpect_contains("EBS_SNAPSHOT 30 days, not the declared 14");
	}

	/// The root email is the base plus-addressed with the account and the
	/// instant, and a base that is not a plain address is refused.
	#[beet_core::test]
	fn plus_addresses_the_root_email() {
		AwsAccount::root_email("pete@example.com", "beetmash", 1791530298828)
			.unwrap()
			.xpect_eq("pete+aws-beetmash-1791530298828@example.com");
		AwsAccount::root_email("pete+x@example.com", "beet", 1).unwrap_err();
		AwsAccount::root_email("example.com", "beet", 1).unwrap_err();
	}

	/// Retention outside what the Recycle Bin accepts is refused before
	/// anybody is asked for a code.
	#[beet_core::test]
	fn refuses_retention_out_of_range() {
		beetmash().check().unwrap();
		AwsAccount {
			volume_retention_days: 8,
			..beetmash()
		}
		.check()
		.unwrap_err()
		.to_string()
		.xpect_contains("volume_retention_days");
		AwsAccount {
			snapshot_retention_days: 0,
			..beetmash()
		}
		.check()
		.unwrap_err();
		AwsAccount {
			unlock_delay_days: 6,
			..beetmash()
		}
		.check()
		.unwrap_err();
	}

	/// An account still holding a stack's state is not closed; one holding
	/// only what a teardown leaves is.
	#[beet_core::test]
	fn refuses_to_close_with_live_stacks() {
		let member = MemberAccount {
			name: "beetmash".into(),
			id: "222".into(),
		};
		AwsAccount::refuse_live_state(&member, &[
			"beetmash-mail--prod--tofu-tfstate".into(),
		])
		.unwrap_err()
		.to_string()
		.xpect_contains("beetmash-mail--prod--tofu-tfstate");
		AwsAccount::refuse_live_state(&member, &[]).unwrap();
	}
}

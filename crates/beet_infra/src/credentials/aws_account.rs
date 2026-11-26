//! `<AwsAccount/>`: the AWS account a repo deploys into, a member of the
//! operator's Organization, and its converge.

use super::aws_deploy_credential::AwsAdmin;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

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
/// With `{IdentityCenter{..}}` declared beside it, the converge also
/// provisions the human path in, as the management account (after step 3,
/// and each member's part right after step 4): the operator group, the two
/// permission sets (`beet-admin`, `AdministratorAccess` for an hour;
/// `beet-read`, `ViewOnlyAccess` for eight), provisioned when one changed,
/// and the group's assignment to both in the member. Per the `sso-admin` and
/// `identitystore` models every one of those has an api, and two steps have
/// none: enabling the Organization's instance and its MFA policy are the
/// console's, so an instance `ListInstances` does not answer fails the
/// converge naming the clicks. The management account itself is never
/// assigned, and a close removes the member's assignments first.
///
/// A rule that exists is left as it is, whatever its retention: a locked rule
/// changes only after an unlock and its delay, which is an operator's act at
/// the console, and the converge says when the two disagree.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct AwsAccount {
	/// The account's name in the Organization, found by it. Required: every
	/// entry deploying into the account names it, ie `name="beet"`.
	pub name: SmolStr,
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
			name: SmolStr::default(),
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

	/// The name, refused when unset: an account is shared by every entry
	/// deploying into it, so no one entry's name is the account's.
	pub(crate) fn resolved_name(&self) -> Result<SmolStr> {
		match self.name.is_empty() {
			false => self.name.clone().xok(),
			true => bevybail!(
				"`<AwsAccount/>` names the account it deploys into: \
				`<AwsAccount name=\"..\"/>`, its name in the Organization"
			),
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
		identity_center: Option<&IdentityCenter>,
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
						&Self::member_guard(),
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
					&Self::agent_read(),
				)
				.await?,
		);
		// 3b. the Identity Center instance's operator group and permission
		// sets, before any member is assigned them
		let sso = match identity_center {
			Some(center) => Some(
				Self::converge_identity_center(calls, center, &mut report)
					.await?,
			),
			None => None,
		};
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
		// 4b. the operator group assigned both sets in the member
		if let Some((instance, group, sets)) = &sso {
			for (spec, set) in sets {
				if !calls.is_assigned(instance, &member.id, set, group).await? {
					calls
						.assign(instance, &member.id, set, group, false)
						.await?;
					report.push(format!(
						"{} assigned `{}` in {}",
						identity_center
							.map(|center| center.group.as_str())
							.unwrap_or_default(),
						spec.name,
						member.name
					));
				}
			}
		}
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

	/// The Identity Center steps in the management account: the instance,
	/// which is the console's to enable, the operator group and the two
	/// permission sets, each found or created, a set provisioned when it
	/// changed. Answers what the member step assigns.
	async fn converge_identity_center<C: ManagementCalls>(
		calls: &C,
		center: &IdentityCenter,
		report: &mut Vec<String>,
	) -> Result<(
		IdentityCenterInstance,
		SmolStr,
		Vec<(PermissionSetSpec, SmolStr)>,
	)> {
		let Some(instance) = calls.sso_instance().await? else {
			bevybail!(
				"the Organization has no IAM Identity Center instance, and \
				enabling one has no api: in the management account's console, \
				IAM Identity Center > Enable (an organization instance, in the \
				`IdentityCenter`'s region), then Settings > Authentication > \
				Multi-factor authentication > Configure: every time they sign \
				in, and let users register their own device. Then rerun the \
				elevated deploy"
			);
		};
		let group = match calls.sso_group(&instance, &center.group).await? {
			Some(group) => group,
			None => {
				let group =
					calls.create_sso_group(&instance, &center.group).await?;
				report.push(format!(
					"identity center group {} created",
					center.group
				));
				group
			}
		};
		let mut sets = Vec::new();
		for spec in IdentityCenter::PERMISSION_SETS {
			let (set, created) = match calls
				.permission_set(&instance, spec.name)
				.await?
			{
				Some(set) => (set, false),
				None => {
					report
						.push(format!("permission set {} created", spec.name));
					(calls.create_permission_set(&instance, &spec).await?, true)
				}
			};
			let attached = calls
				.ensure_set_policy(&instance, &set, spec.policy_arn)
				.await?;
			if attached {
				report.push(format!(
					"permission set {} holds {}",
					spec.name, spec.policy_arn
				));
			}
			if created || attached {
				calls.provision_set(&instance, &set).await?;
				report
					.push(format!("permission set {} provisioned", spec.name));
			}
			sets.push((spec, set));
		}
		(instance, group, sets).xok()
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
		value!({
			"Version": "2012-10-17",
			"Statement": [{
				"Sid": "MembersStay",
				"Effect": "Deny",
				"Action": ["organizations:LeaveOrganization", "account:CloseAccount"],
				"Resource": "*",
			}],
		})
	}

	/// `beet-agent`'s path to a member: finding it by name, assuming its
	/// read role, and finding the Identity Center instance a sign in starts
	/// at. Granted here rather than left to `ViewOnlyAccess`'s
	/// contents, so the read path never depends on what AWS puts in it.
	fn agent_read() -> Value {
		value!({
			"Version": "2012-10-17",
			"Statement": [
				{
					"Sid": "FindMembers",
					"Effect": "Allow",
					"Action": "organizations:ListAccounts",
					"Resource": "*",
				},
				{
					"Sid": "FindIdentityCenter",
					"Effect": "Allow",
					"Action": "sso:ListInstances",
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
	/// `organizations:CloseAccount` as the management account.
	async fn close_declared(caller: &AsyncEntity) -> Result<String> {
		let Some((account, region)) = Self::declared(caller).await? else {
			bevybail!("this launch declares no `<AwsAccount/>` to close");
		};
		let name = account.resolved_name()?;
		let session = AdminSession::acquire(AdminSession::DEFAULT_SECS).await?;
		let management = AwsAdmin::new(session.credentials().clone());
		let member =
			MemberAccount::find_in(&management.organizations(), &name).await?;
		let inside = management.acting_as(
			member
				.assume_with(&management.sts(), MemberAccount::ADMIN_ROLE)
				.await?,
		);
		let keys = inside
			.state_keys(&S3Backend::default_bucket(&member.id), &region)
			.await?;
		Self::refuse_live_state(&member, &keys)?;
		// the operator group's assignments go first, so the closed account
		// leaves nothing behind in the access portal
		if let Some((center, sso_region)) =
			IdentityCenter::declared(caller).await?
		{
			let sso = IdentityCenterAdmin::from_clients(
				management.client(AwsService::SSO_ADMIN, &sso_region),
				management.client(AwsService::IDENTITY_STORE, &sso_region),
			);
			if let Some(instance) = sso.instance().await?
				&& let Some(group) =
					sso.find_group(&instance, &center.group).await?
			{
				for spec in IdentityCenter::PERMISSION_SETS {
					if let Some(set) =
						sso.find_permission_set(&instance, spec.name).await?
						&& sso
							.is_assigned(&instance, &member.id, &set, &group)
							.await?
					{
						sso.assign(&instance, &member.id, &set, &group, true)
							.await?;
					}
				}
			}
		}
		management
			.organizations()
			.call_json("CloseAccount", &value!({"AccountId": &member.id}))
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
/// [`AwsManagement`] is AWS's.
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
		document: &Value,
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
		document: &Value,
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
	/// The Organization's Identity Center instance, `None` until enabled.
	async fn sso_instance(&self) -> Result<Option<IdentityCenterInstance>>;
	/// The identity store group named `name`.
	async fn sso_group(
		&self,
		instance: &IdentityCenterInstance,
		name: &str,
	) -> Result<Option<SmolStr>>;
	async fn create_sso_group(
		&self,
		instance: &IdentityCenterInstance,
		name: &str,
	) -> Result<SmolStr>;
	/// The permission set named `name`, by arn.
	async fn permission_set(
		&self,
		instance: &IdentityCenterInstance,
		name: &str,
	) -> Result<Option<SmolStr>>;
	async fn create_permission_set(
		&self,
		instance: &IdentityCenterInstance,
		spec: &PermissionSetSpec,
	) -> Result<SmolStr>;
	/// Attach `policy` to `set` unless it holds it, answering whether it did.
	async fn ensure_set_policy(
		&self,
		instance: &IdentityCenterInstance,
		set: &str,
		policy: &str,
	) -> Result<bool>;
	async fn provision_set(
		&self,
		instance: &IdentityCenterInstance,
		set: &str,
	) -> Result;
	/// Whether `group` holds `set` in `account`.
	async fn is_assigned(
		&self,
		instance: &IdentityCenterInstance,
		account: &str,
		set: &str,
		group: &str,
	) -> Result<bool>;
	/// Assign `group` `set` in `account`, or with `remove` take it away.
	async fn assign(
		&self,
		instance: &IdentityCenterInstance,
		account: &str,
		set: &str,
		group: &str,
		remove: bool,
	) -> Result;
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

/// The management account's administrator session making the calls at AWS,
/// see [`ManagementCalls`].
pub(crate) struct AwsManagement {
	admin: AwsAdmin,
	region: SmolStr,
	/// The Identity Center instance's region, where one is declared.
	sso_region: Option<SmolStr>,
}

impl AwsManagement {
	/// The service control policy type, the one kind of policy the guard is.
	const SCP: &'static str = "SERVICE_CONTROL_POLICY";

	pub(crate) fn new(admin: AwsAdmin, region: impl Into<SmolStr>) -> Self {
		Self {
			admin,
			region: region.into(),
			sso_region: None,
		}
	}

	/// Reach the Identity Center instance in `region`.
	pub(crate) fn with_sso_region(
		mut self,
		region: impl Into<SmolStr>,
	) -> Self {
		self.sso_region = Some(region.into());
		self
	}

	/// The Identity Center administration api, as this administrator.
	fn identity_center(&self) -> Result<IdentityCenterAdmin> {
		let region = self.sso_region.as_deref().ok_or_else(|| {
			bevyhow!("no `IdentityCenter` is declared to reach")
		})?;
		IdentityCenterAdmin::from_clients(
			self.admin.client(AwsService::SSO_ADMIN, region),
			self.admin.client(AwsService::IDENTITY_STORE, region),
		)
		.xok()
	}

	/// An Organizations `action` with `body`.
	async fn organizations(&self, action: &str, body: Value) -> Result<Value> {
		self.admin.organizations().call_json(action, &body).await
	}

	/// Every entry of the `list` an Organizations `action` answers, across
	/// its pages.
	async fn organizations_list(
		&self,
		action: &str,
		body: Value,
		list: &str,
	) -> Result<Vec<Value>> {
		let organizations = self.admin.organizations();
		let mut entries = Vec::new();
		let mut body = body;
		loop {
			let mut page = organizations.call_json(action, &body).await?;
			if let Some(held) =
				page.get_mut(list).and_then(|held| held.as_list_mut().ok())
			{
				entries.append(held);
			}
			match page["NextToken"].as_str().ok() {
				Some(token) => body.insert("NextToken", token)?,
				None => break,
			};
		}
		entries.xok()
	}
}

impl ManagementCalls for AwsManagement {
	type Member = AwsMember;

	async fn organization_exists(&self) -> Result<bool> {
		match self.organizations("DescribeOrganization", value!({})).await {
			Ok(_) => true.xok(),
			Err(err)
				if AwsError::is(&err, &[
					"AWSOrganizationsNotInUseException",
				]) =>
			{
				false.xok()
			}
			Err(err) => Err(err),
		}
	}

	async fn create_organization(&self) -> Result {
		self.organizations("CreateOrganization", value!({"FeatureSet": "ALL"}))
			.await?;
		OK
	}

	async fn root(&self) -> Result<(SmolStr, bool)> {
		let roots = self.organizations("ListRoots", value!({})).await?;
		let root = &roots["Roots"][0];
		let id = root["Id"]
			.as_str()
			.ok()
			.ok_or_else(|| bevyhow!("the Organization answered no root"))?;
		let enabled = root["PolicyTypes"]
			.as_list()
			.ok()
			.into_iter()
			.flatten()
			.any(|policy_type| {
				policy_type["Type"].as_str().ok() == Some(Self::SCP)
					&& policy_type["Status"].as_str().ok() == Some("ENABLED")
			});
		(SmolStr::new(id), enabled).xok()
	}

	async fn enable_policies(&self, root: &str) -> Result {
		self.organizations(
			"EnablePolicyType",
			value!({"RootId": root, "PolicyType": Self::SCP}),
		)
		.await?;
		OK
	}

	async fn find_policy(&self, name: &str) -> Result<Option<SmolStr>> {
		self.organizations_list(
			"ListPolicies",
			value!({"Filter": Self::SCP}),
			"Policies",
		)
		.await?
		.iter()
		.find(|policy| policy["Name"].as_str().ok() == Some(name))
		.and_then(|policy| policy["Id"].as_str().ok())
		.map(SmolStr::new)
		.xok()
	}

	async fn create_policy(
		&self,
		name: &str,
		document: &Value,
	) -> Result<SmolStr> {
		let created = self
			.organizations(
				"CreatePolicy",
				value!({
					"Name": name,
					"Type": Self::SCP,
					"Description": "members stay in the Organization; closing is the management account's act",
					// the api takes the policy as json text
					"Content": document.to_json_string(),
				}),
			)
			.await?;
		created["Policy"]["PolicySummary"]["Id"]
			.as_str()
			.ok()
			.map(SmolStr::new)
			.ok_or_else(|| {
				bevyhow!("`organizations:CreatePolicy` answered no id")
			})
	}

	async fn attached_policies(&self, target: &str) -> Result<Vec<SmolStr>> {
		self.organizations_list(
			"ListPoliciesForTarget",
			value!({"TargetId": target, "Filter": Self::SCP}),
			"Policies",
		)
		.await?
		.iter()
		.filter_map(|policy| policy["Id"].as_str().ok())
		.map(SmolStr::new)
		.collect::<Vec<_>>()
		.xok()
	}

	async fn attach_policy(&self, policy: &str, target: &str) -> Result {
		// a policy type just enabled is `PENDING_ENABLE` for a moment, and an
		// attach then answers `PolicyTypeNotEnabled`
		let mut last = None;
		for _ in 0..10 {
			match self
				.organizations(
					"AttachPolicy",
					value!({"PolicyId": policy, "TargetId": target}),
				)
				.await
			{
				Ok(_) => return OK,
				Err(err)
					if AwsError::is(&err, &[
						"PolicyTypeNotEnabledException",
					]) =>
				{
					last = Some(err);
					self.admin.pause(3).await;
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
		document: &Value,
	) -> Result<String> {
		let account = self.admin.account_id().await?;
		let report = AwsDeployPolicy::converge_document(
			&self.admin,
			&account,
			name,
			document,
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
		MemberAccount::lookup_in(&self.admin.organizations(), name)
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
		let created = self
			.organizations(
				"CreateAccount",
				value!({
					"Email": email,
					"AccountName": name,
					"RoleName": MemberAccount::ADMIN_ROLE,
				}),
			)
			.await?;
		created["CreateAccountStatus"]["Id"]
			.as_str()
			.ok()
			.map(SmolStr::new)
			.ok_or_else(|| {
				bevyhow!("`organizations:CreateAccount` answered no request id")
			})
	}

	async fn account_created(&self, request: &str) -> Result<Option<SmolStr>> {
		let answer = self
			.organizations(
				"DescribeCreateAccountStatus",
				value!({"CreateAccountRequestId": request}),
			)
			.await?;
		let status = &answer["CreateAccountStatus"];
		match (
			status["State"].as_str().ok(),
			status["AccountId"].as_str().ok(),
		) {
			(Some("SUCCEEDED"), Some(id)) => Some(SmolStr::new(id)).xok(),
			(Some("IN_PROGRESS"), _) => None.xok(),
			(Some("FAILED"), _) => bevybail!(
				"creating the account failed: {}",
				status["FailureReason"]
					.as_str()
					.ok()
					.unwrap_or("no reason given")
			),
			(state, _) => bevybail!(
				"creating the account answered the state `{}`",
				state.unwrap_or_default()
			),
		}
	}

	async fn wait(&self) { self.admin.pause(5).await; }

	async fn sso_instance(&self) -> Result<Option<IdentityCenterInstance>> {
		self.identity_center()?.instance().await
	}

	async fn sso_group(
		&self,
		instance: &IdentityCenterInstance,
		name: &str,
	) -> Result<Option<SmolStr>> {
		self.identity_center()?.find_group(instance, name).await
	}

	async fn create_sso_group(
		&self,
		instance: &IdentityCenterInstance,
		name: &str,
	) -> Result<SmolStr> {
		self.identity_center()?.create_group(instance, name).await
	}

	async fn permission_set(
		&self,
		instance: &IdentityCenterInstance,
		name: &str,
	) -> Result<Option<SmolStr>> {
		self.identity_center()?
			.find_permission_set(instance, name)
			.await
	}

	async fn create_permission_set(
		&self,
		instance: &IdentityCenterInstance,
		spec: &PermissionSetSpec,
	) -> Result<SmolStr> {
		self.identity_center()?
			.create_permission_set(instance, spec)
			.await
	}

	async fn ensure_set_policy(
		&self,
		instance: &IdentityCenterInstance,
		set: &str,
		policy: &str,
	) -> Result<bool> {
		self.identity_center()?
			.ensure_policy(instance, set, policy)
			.await
	}

	async fn provision_set(
		&self,
		instance: &IdentityCenterInstance,
		set: &str,
	) -> Result {
		self.identity_center()?.provision(instance, set).await
	}

	async fn is_assigned(
		&self,
		instance: &IdentityCenterInstance,
		account: &str,
		set: &str,
		group: &str,
	) -> Result<bool> {
		self.identity_center()?
			.is_assigned(instance, account, set, group)
			.await
	}

	async fn assign(
		&self,
		instance: &IdentityCenterInstance,
		account: &str,
		set: &str,
		group: &str,
		remove: bool,
	) -> Result {
		self.identity_center()?
			.assign(instance, account, set, group, remove)
			.await
	}

	async fn enter(&self, member: &MemberAccount) -> Result<AwsMember> {
		// a new account's role is assumable a few seconds after it exists
		let sts = self.admin.sts();
		let mut last = None;
		for _ in 0..12 {
			match member.assume_with(&sts, MemberAccount::ADMIN_ROLE).await {
				Ok(credentials) => {
					return AwsMember {
						admin: self.admin.acting_as(credentials),
						region: self.region.clone(),
					}
					.xok();
				}
				Err(err) => {
					last = Some(err);
					self.admin.pause(5).await;
				}
			}
		}
		Err(last.unwrap_or_else(|| bevyhow!("the member was never entered")))
	}
}

/// The administrator role inside a member making the calls at AWS, see
/// [`MemberCalls`].
pub(crate) struct AwsMember {
	admin: AwsAdmin,
	region: SmolStr,
}

impl AwsMember {
	/// The session inside the member, for the deploy credential's converge.
	pub(crate) fn into_admin(self) -> AwsAdmin { self.admin }

	/// The Recycle Bin in the member's region.
	fn recycle_bin(&self) -> AwsClient {
		self.admin.client(AwsService::RBIN, &self.region)
	}
}

impl MemberCalls for AwsMember {
	async fn find_rule(
		&self,
		resource_type: &str,
		description: &str,
	) -> Result<Option<u16>> {
		let recycle_bin = self.recycle_bin();
		let mut body = value!({"ResourceType": resource_type});
		loop {
			let page = recycle_bin
				.send_json(
					AwsRequest::post("ListRules", "/list-rules")
						.with_json(&body)?,
				)
				.await?;
			if let Some(rule) =
				page["Rules"].as_list().ok().into_iter().flatten().find(
					|rule| {
						rule["Description"].as_str().ok() == Some(description)
					},
				) {
				return rule["RetentionPeriod"]["RetentionPeriodValue"]
					.as_u64()
					.ok()
					.map(u16::try_from)
					.transpose()?
					.xok();
			}
			match page["NextToken"].as_str().ok() {
				Some(token) => body.insert("NextToken", token)?,
				None => return None.xok(),
			};
		}
	}

	async fn create_rule(
		&self,
		resource_type: &str,
		description: &str,
		days: u16,
		unlock_delay_days: u16,
	) -> Result {
		self.recycle_bin()
			.send(AwsRequest::post("CreateRule", "/rules").with_json(
				&value!({
					"ResourceType": resource_type,
					"Description": description,
					"RetentionPeriod": {
						"RetentionPeriodValue": days,
						"RetentionPeriodUnit": "DAYS",
					},
					"LockConfiguration": {
						"UnlockDelay": {
							"UnlockDelayValue": unlock_delay_days,
							"UnlockDelayUnit": "DAYS",
						},
					},
				}),
			)?)
			.await?;
		OK
	}

	async fn role_exists(&self, name: &str) -> Result<bool> {
		self.admin
			.iam_option("GetRole", &[("RoleName", name)])
			.await
			.map(|found| found.is_some())
	}

	async fn create_read_role(&self, name: &str, principal: &str) -> Result {
		let trust = value!({
			"Version": "2012-10-17",
			"Statement": [{
				"Effect": "Allow",
				"Principal": { "AWS": principal },
				"Action": "sts:AssumeRole",
			}],
		})
		.to_json_string();
		self.admin
			.iam_call("CreateRole", &[
				("RoleName", name),
				("AssumeRolePolicyDocument", trust.as_str()),
				("Tags.member.1.Key", "beet"),
				("Tags.member.1.Value", "agent-read"),
			])
			.await?;
		self.admin
			.iam_call("AttachRolePolicy", &[
				("RoleName", name),
				(
					"PolicyArn",
					"arn:aws:iam::aws:policy/job-function/ViewOnlyAccess",
				),
			])
			.await
	}

	async fn bucket_exists(&self, bucket: &str) -> Result<bool> {
		self.admin.bucket(bucket, &self.region).store_exists().await
	}

	async fn create_state_bucket(&self, bucket: &str, region: &str) -> Result {
		let store = self.admin.bucket(bucket, region);
		store.store_create().await?;
		// every apply overwrites the state in place, so a version is its only
		// undo, as for a bucket a first deploy creates
		store.set_object_versioning().await?;
		store.block_public_access().await
	}
}

#[cfg(test)]
mod test {
	use super::super::aws_deploy_credential::aws_seam;
	use super::*;

	/// The calls a converge made, in order, against an account where nothing
	/// exists yet (`fresh`) or where everything already does.
	#[derive(Clone, Copy)]
	struct Fake {
		fresh: bool,
		/// What the Organization's Identity Center holds.
		sso: SsoFake,
		log: Store<Vec<String>>,
	}

	/// The Identity Center states a converge meets.
	#[derive(Clone, Copy, PartialEq, Eq)]
	enum SsoFake {
		/// Never enabled at the console.
		NoInstance,
		/// Enabled, nothing provisioned.
		Empty,
		/// The group and both sets in place, assigned in an account that
		/// existed.
		Provisioned,
	}

	impl Fake {
		fn new(fresh: bool) -> Self {
			Self {
				fresh,
				sso: SsoFake::Provisioned,
				log: Store::default(),
			}
		}

		fn with_sso(mut self, sso: SsoFake) -> Self {
			self.sso = sso;
			self
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
			document: &Value,
		) -> Result<SmolStr> {
			self.call(format!("create-policy {name}"));
			document.xpect_eq(AwsAccount::member_guard());
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
			_document: &Value,
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
		async fn sso_instance(&self) -> Result<Option<IdentityCenterInstance>> {
			(self.sso != SsoFake::NoInstance)
				.then(|| IdentityCenterInstance {
					arn: "arn:ssoins".into(),
					identity_store_id: "d-123".into(),
				})
				.xok()
		}
		async fn sso_group(
			&self,
			_instance: &IdentityCenterInstance,
			_name: &str,
		) -> Result<Option<SmolStr>> {
			(self.sso == SsoFake::Provisioned)
				.then(|| SmolStr::new("g-ops"))
				.xok()
		}
		async fn create_sso_group(
			&self,
			_instance: &IdentityCenterInstance,
			name: &str,
		) -> Result<SmolStr> {
			self.call(format!("create-group {name}"));
			SmolStr::new("g-ops").xok()
		}
		async fn permission_set(
			&self,
			_instance: &IdentityCenterInstance,
			name: &str,
		) -> Result<Option<SmolStr>> {
			(self.sso == SsoFake::Provisioned)
				.then(|| SmolStr::new(format!("ps-{name}")))
				.xok()
		}
		async fn create_permission_set(
			&self,
			_instance: &IdentityCenterInstance,
			spec: &PermissionSetSpec,
		) -> Result<SmolStr> {
			self.call(format!(
				"create-permission-set {} {}",
				spec.name, spec.session
			));
			SmolStr::new(format!("ps-{}", spec.name)).xok()
		}
		async fn ensure_set_policy(
			&self,
			_instance: &IdentityCenterInstance,
			set: &str,
			policy: &str,
		) -> Result<bool> {
			if self.sso == SsoFake::Provisioned {
				return false.xok();
			}
			self.call(format!("attach-set-policy {set} {policy}"));
			true.xok()
		}
		async fn provision_set(
			&self,
			_instance: &IdentityCenterInstance,
			set: &str,
		) -> Result {
			self.call(format!("provision {set}"));
			OK
		}
		async fn is_assigned(
			&self,
			_instance: &IdentityCenterInstance,
			_account: &str,
			_set: &str,
			_group: &str,
		) -> Result<bool> {
			(self.sso == SsoFake::Provisioned && !self.fresh).xok()
		}
		async fn assign(
			&self,
			_instance: &IdentityCenterInstance,
			account: &str,
			set: &str,
			group: &str,
			remove: bool,
		) -> Result {
			self.call(format!("assign {account} {set} {group} {remove}"));
			OK
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
			name: "beetmash".into(),
			..default()
		}
	}

	/// From nothing, the converge goes in the only order that works: the
	/// Organization, its guard enabled then attached, the agent's read path,
	/// the account created and polled, and only then anything inside it.
	#[beet_core::test]
	async fn converges_in_order() {
		let fake = Fake::new(true);
		let (member, _, report) = beetmash()
			.converge(&fake, "ap-southeast-2", None)
			.await
			.unwrap();
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
		let (_, _, report) = beetmash()
			.converge(&fake, "ap-southeast-2", None)
			.await
			.unwrap();
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
	/// anybody is asked for a code, as is an account with no name.
	#[beet_core::test]
	fn refuses_retention_out_of_range() {
		beetmash().check().unwrap();
		AwsAccount::default()
			.resolved_name()
			.unwrap_err()
			.to_string()
			.xpect_contains("name=");
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

	/// The management calls at a stand-in AWS answering `answer`.
	fn management(
		answer: impl 'static + Send + Sync + Fn(&aws_seam::Asked) -> Response,
	) -> (AwsManagement, aws_seam::Log) {
		let (admin, log) = aws_seam::admin(answer);
		(AwsManagement::new(admin, "ap-southeast-2"), log)
	}

	/// The member calls at a stand-in AWS answering `answer`.
	fn member(
		answer: impl 'static + Send + Sync + Fn(&aws_seam::Asked) -> Response,
	) -> (AwsMember, aws_seam::Log) {
		let (admin, log) = aws_seam::admin(answer);
		(
			AwsMember {
				admin,
				region: "ap-southeast-2".into(),
			},
			log,
		)
	}

	/// The root is the first, and its guard's policy type counts as enabled
	/// only once its status says so.
	#[beet_core::test]
	async fn reads_the_root_and_its_policy_status() {
		let roots = |status: &'static str| {
			move |_: &aws_seam::Asked| {
				aws_seam::ok(format!(
					r#"{{"Roots":[{{"Id":"r-ab12","Name":"Root","PolicyTypes":[
					{{"Type":"TAG_POLICY","Status":"ENABLED"}},
					{{"Type":"SERVICE_CONTROL_POLICY","Status":"{status}"}}]}}]}}"#
				))
			}
		};
		let (calls, log) = management(roots("ENABLED"));
		calls
			.root()
			.await
			.unwrap()
			.xpect_eq((SmolStr::new("r-ab12"), true));
		let asked = log.lock().unwrap()[0].clone();
		asked.target.xpect_eq("AWSOrganizationsV20161128.ListRoots");
		asked
			.line
			.xpect_eq("POST https://organizations.us-east-1.amazonaws.com/");
		let (calls, _) = management(roots("PENDING_ENABLE"));
		calls.root().await.unwrap().1.xpect_false();
	}

	/// A creation is in progress, then succeeded with its account, and a
	/// failure says why.
	#[beet_core::test]
	async fn reads_the_create_account_status() {
		let status = |body: &'static str| {
			move |_: &aws_seam::Asked| {
				aws_seam::ok(format!(r#"{{"CreateAccountStatus":{body}}}"#))
			}
		};
		let (calls, log) =
			management(status(r#"{"Id":"car-1","State":"IN_PROGRESS"}"#));
		calls.account_created("car-1").await.unwrap().xpect_none();
		log.lock().unwrap()[0]
			.json()
			.xpect_eq(value!({"CreateAccountRequestId": "car-1"}));
		let (calls, _) = management(status(
			r#"{"Id":"car-1","State":"SUCCEEDED","AccountId":"222"}"#,
		));
		calls
			.account_created("car-1")
			.await
			.unwrap()
			.xpect_eq(Some(SmolStr::new("222")));
		let (calls, _) = management(status(
			r#"{"Id":"car-1","State":"FAILED","FailureReason":"EMAIL_ALREADY_EXISTS"}"#,
		));
		calls
			.account_created("car-1")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("EMAIL_ALREADY_EXISTS");
	}

	/// An attach right after enabling the policy type is refused for a
	/// moment, and retried until the type is enabled.
	#[beet_core::test]
	async fn retries_an_attach_while_the_policy_type_enables() {
		// refused twice, then attached
		let refusals = std::sync::Mutex::new(2);
		let (calls, log) = management(move |_| {
			let mut left = refusals.lock().unwrap();
			match *left {
				0 => aws_seam::ok(""),
				_ => {
					*left -= 1;
					aws_seam::json_error("PolicyTypeNotEnabledException")
				}
			}
		});
		calls.attach_policy("p-guard", "r-ab12").await.unwrap();
		let log = log.lock().unwrap().clone();
		log.len().xpect_eq(3);
		log[2]
			.json()
			.xpect_eq(value!({"PolicyId": "p-guard", "TargetId": "r-ab12"}));
		// any other refusal is answered at once
		let (calls, log) =
			management(|_| aws_seam::json_error("PolicyNotFoundException"));
		calls.attach_policy("p-guard", "r-ab12").await.unwrap_err();
		log.lock().unwrap().len().xpect_eq(1);
	}

	/// The guard reaches the api as json text, the document itself rather
	/// than its human display, which is no json at all.
	#[beet_core::test]
	async fn creates_the_guard_as_json() {
		let (calls, log) = management(|_| {
			aws_seam::ok(r#"{"Policy":{"PolicySummary":{"Id":"p-guard"}}}"#)
		});
		calls
			.create_policy(AwsAccount::MEMBER_GUARD, &AwsAccount::member_guard())
			.await
			.unwrap()
			.xpect_eq(SmolStr::new("p-guard"));
		let asked = log.lock().unwrap()[0].clone();
		asked.target.xpect_eq("AWSOrganizationsV20161128.CreatePolicy");
		Value::parse_json(asked.json()["Content"].as_str().unwrap())
			.unwrap()
			.xpect_eq(AwsAccount::member_guard());
	}

	/// The guard is found by name across pages of the account's policies.
	#[beet_core::test]
	async fn finds_the_guard_across_pages() {
		let (calls, log) = management(|asked| match asked.json()["NextToken"]
			.as_str()
			.ok()
		{
			None => aws_seam::ok(
				r#"{"Policies":[{"Id":"p-full","Name":"FullAWSAccess"}],"NextToken":"t1"}"#,
			),
			Some(_) => aws_seam::ok(
				r#"{"Policies":[{"Id":"p-guard","Name":"beet-member-guard"}]}"#,
			),
		});
		calls
			.find_policy(AwsAccount::MEMBER_GUARD)
			.await
			.unwrap()
			.xpect_eq(Some(SmolStr::new("p-guard")));
		let log = log.lock().unwrap().clone();
		log[0]
			.json()
			.xpect_eq(value!({"Filter": "SERVICE_CONTROL_POLICY"}));
		log[1].json().xpect_eq(value!({
			"Filter": "SERVICE_CONTROL_POLICY",
			"NextToken": "t1",
		}));
	}

	/// A rule is the one carrying the converge's description, across pages,
	/// and its retention is read; a rule is created locked.
	#[beet_core::test]
	async fn finds_and_creates_recycle_bin_rules() {
		let (inside, log) = member(|asked| {
			match asked.json()["NextToken"].as_str().ok() {
				None => aws_seam::ok(
					r#"{"Rules":[{"Identifier":"a","Description":"someone else's","RetentionPeriod":{"RetentionPeriodValue":3,"RetentionPeriodUnit":"DAYS"}}],"NextToken":"t1"}"#,
				),
				Some(_) => aws_seam::ok(format!(
					r#"{{"Rules":[{{"Identifier":"b","Description":"{}","RetentionPeriod":{{"RetentionPeriodValue":14,"RetentionPeriodUnit":"DAYS"}},"LockState":"locked"}}]}}"#,
					AwsAccount::RULE_DESCRIPTION
				)),
			}
		});
		inside
			.find_rule("EBS_SNAPSHOT", AwsAccount::RULE_DESCRIPTION)
			.await
			.unwrap()
			.xpect_eq(Some(14));
		let log = log.lock().unwrap().clone();
		log[0].line.xpect_eq(
			"POST https://rbin.ap-southeast-2.amazonaws.com/list-rules",
		);
		log[1].json().xpect_eq(value!({
			"ResourceType": "EBS_SNAPSHOT",
			"NextToken": "t1",
		}));

		let (inside, log) = member(|_| aws_seam::ok(r#"{"Identifier":"c"}"#));
		inside
			.create_rule("EBS_VOLUME", AwsAccount::RULE_DESCRIPTION, 7, 10)
			.await
			.unwrap();
		let asked = log.lock().unwrap()[0].clone();
		asked
			.line
			.xpect_eq("POST https://rbin.ap-southeast-2.amazonaws.com/rules");
		asked.json().xpect_eq(value!({
			"ResourceType": "EBS_VOLUME",
			"Description": AwsAccount::RULE_DESCRIPTION,
			"RetentionPeriod": {"RetentionPeriodValue": 7, "RetentionPeriodUnit": "DAYS"},
			"LockConfiguration": {
				"UnlockDelay": {"UnlockDelayValue": 10, "UnlockDelayUnit": "DAYS"},
			},
		}));
	}

	/// The state bucket is created in its region, then versioned, then closed
	/// to the public, in that order.
	#[beet_core::test]
	async fn creates_the_state_bucket() {
		let (endpoint, requests) = aws_seam::s3_server();
		let (inside, _) = member(|_| aws_seam::ok(""));
		let inside = AwsMember {
			admin: aws_seam::with_s3_endpoint(inside.admin, &endpoint),
			..inside
		};
		inside
			.create_state_bucket("beet-state-222", "ap-southeast-2")
			.await
			.unwrap();
		let requests = requests.lock().unwrap().clone();
		requests
			.iter()
			.map(|(line, _)| line.clone())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"PUT /beet-state-222/".to_string(),
				"PUT /beet-state-222/?versioning".into(),
				"PUT /beet-state-222/?publicAccessBlock".into(),
			]);
		requests[0].1.as_str().xpect_contains(
			"<LocationConstraint>ap-southeast-2</LocationConstraint>",
		);
		requests[1]
			.1
			.as_str()
			.xpect_contains("<Status>Enabled</Status>");
		requests[2].1.as_str().xpect_contains(
			"<RestrictPublicBuckets>true</RestrictPublicBuckets>",
		);
	}

	/// The Identity Center lines a converge with an instance declared logs,
	/// the account's own lines left out.
	async fn sso_log(fake: Fake) -> Vec<String> {
		beetmash()
			.converge(&fake, "ap-southeast-2", Some(&IdentityCenter::default()))
			.await
			.unwrap();
		fake.log
			.get()
			.into_iter()
			.filter(|line| {
				[
					"create-group",
					"create-permission-set",
					"attach-set-policy",
					"provision",
					"assign",
				]
				.iter()
				.any(|prefix| line.starts_with(prefix))
			})
			.collect()
	}

	/// The whole provisioning, in order, for a fresh Organization: the group,
	/// each set created, given its policy and provisioned, then the new
	/// account assigned both.
	#[beet_core::test]
	async fn provisions_identity_center_from_nothing() {
		sso_log(Fake::new(true).with_sso(SsoFake::Empty)).await.xpect_eq(vec![
			"create-group beet-operators".to_string(),
			"create-permission-set beet-admin PT1H".into(),
			"attach-set-policy ps-beet-admin arn:aws:iam::aws:policy/AdministratorAccess".into(),
			"provision ps-beet-admin".into(),
			"create-permission-set beet-read PT8H".into(),
			"attach-set-policy ps-beet-read arn:aws:iam::aws:policy/job-function/ViewOnlyAccess".into(),
			"provision ps-beet-read".into(),
			"assign 222 ps-beet-admin g-ops false".into(),
			"assign 222 ps-beet-read g-ops false".into(),
		]);
	}

	/// An instance enabled but empty, beside an existing account, gains the
	/// same, the account's assignments included.
	#[beet_core::test]
	async fn provisions_an_empty_instance() {
		let log = sso_log(Fake::new(false).with_sso(SsoFake::Empty)).await;
		log.len().xpect_eq(9);
		log[0].as_str().xpect_eq("create-group beet-operators");
	}

	/// A provisioned instance and an assigned account change nothing.
	#[beet_core::test]
	async fn a_provisioned_instance_changes_nothing() {
		sso_log(Fake::new(false).with_sso(SsoFake::Provisioned))
			.await
			.xpect_eq(Vec::<String>::new());
	}

	/// A new member account gains its two assignments and nothing else.
	#[beet_core::test]
	async fn a_new_account_gains_its_assignments() {
		sso_log(Fake::new(true).with_sso(SsoFake::Provisioned))
			.await
			.xpect_eq(vec![
				"assign 222 ps-beet-admin g-ops false".to_string(),
				"assign 222 ps-beet-read g-ops false".into(),
			]);
	}

	/// An instance nobody enabled fails naming the console clicks, before
	/// any member is touched.
	#[beet_core::test]
	async fn an_unenabled_instance_names_the_console() {
		let fake = Fake::new(true).with_sso(SsoFake::NoInstance);
		let Err(err) = beetmash()
			.converge(&fake, "ap-southeast-2", Some(&IdentityCenter::default()))
			.await
		else {
			panic!("an unenabled instance converged");
		};
		err.to_string()
			.xpect_contains("IAM Identity Center > Enable")
			.xpect_contains("Multi-factor authentication");
		fake.log
			.get()
			.iter()
			.any(|line| line.starts_with("create-account"))
			.xpect_false();
	}
}

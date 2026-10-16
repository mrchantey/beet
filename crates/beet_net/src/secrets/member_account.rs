//! A member account of the operator's AWS Organization, and the two roles
//! that reach it from the management account.

use crate::prelude::*;
use beet_core::prelude::*;

/// A member account of the operator's AWS Organization, found by its name:
/// the account a repo deploys into, reached from the management account by
/// assuming a role in it rather than by any credential of its own.
///
/// Two roles reach it. [`ADMIN_ROLE`](Self::ADMIN_ROLE) is the one
/// `organizations create-account` puts there for the management account's
/// administrators, so the `beet-admin` session chains into it with no second
/// code; [`READ_ROLE`](Self::READ_ROLE) is the one the account's converge adds
/// for [`AgentIdentity::USER`], `ViewOnlyAccess` and nothing else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberAccount {
	/// The account's name in the Organization, the repo's.
	pub name: SmolStr,
	/// The id AWS assigned it.
	pub id: SmolStr,
}

impl MemberAccount {
	/// The role every member account holds for the management account's
	/// administrators, created with the account.
	pub const ADMIN_ROLE: &'static str = "OrganizationAccountAccessRole";
	/// The role every member account holds for [`AgentIdentity::USER`]'s
	/// reads.
	pub const READ_ROLE: &'static str = "beet-agent-read";
	/// How long a session in a member account lasts: STS caps a role assumed
	/// from another role's session at an hour.
	pub const SESSION_SECS: i64 = 3600;

	/// The active member account named `name`, asked as `vars`: the
	/// administrator session, or `beet-agent` through its read policy.
	pub async fn find(vars: &[(SmolStr, SmolStr)], name: &str) -> Result<Self> {
		Self::lookup(vars, name).await?.ok_or_else(|| {
			bevyhow!(
				"the Organization holds no active account named `{name}`: an \
				elevated deploy of the package declaring `<AwsAccount \
				name=\"{name}\"/>` creates it"
			)
		})
	}

	/// [`find`](Self::find), `None` when there is no such account.
	pub async fn lookup(
		vars: &[(SmolStr, SmolStr)],
		name: &str,
	) -> Result<Option<Self>> {
		Self::check_name(name)?;
		let query = format!("Accounts[?Name=='{name}' && Status=='ACTIVE'].Id");
		let out = AgentIdentity::aws(vars, [
			"organizations",
			"list-accounts",
			"--query",
			query.as_str(),
			"--output",
			"text",
		])
		.run_async_stdout()
		.await?;
		match out.split_whitespace().collect::<Vec<_>>().as_slice() {
			[] => None.xok(),
			[id] => Some(Self {
				name: name.into(),
				id: (*id).into(),
			})
			.xok(),
			ids => bevybail!(
				"the Organization holds {} active accounts named `{name}` \
				({}), so the name finds none of them",
				ids.len(),
				ids.join(", ")
			),
		}
	}

	/// A name the account lookup can quote safely, which a repo's always is.
	fn check_name(name: &str) -> Result {
		match !name.is_empty()
			&& name.chars().all(|char| {
				char.is_ascii_alphanumeric() || "-_.".contains(char)
			}) {
			true => OK,
			false => bevybail!(
				"`{name}` is not an account name: a member account is named \
				for its repo, alphanumeric with `-`, `_` or `.`"
			),
		}
	}

	/// Assume `role` in this account as `vars`, for the hour a member session
	/// lasts.
	pub async fn assume(
		&self,
		vars: &[(SmolStr, SmolStr)],
		role: &str,
	) -> Result<AdminSession> {
		let arn = format!("arn:aws:iam::{}:role/{role}", self.id);
		let seconds = Self::SESSION_SECS.to_string();
		let body = AgentIdentity::aws(vars, [
			"sts",
			"assume-role",
			"--role-arn",
			arn.as_str(),
			"--role-session-name",
			AgentIdentity::USER,
			"--duration-seconds",
			seconds.as_str(),
			"--query",
			"Credentials.[AccessKeyId,SecretAccessKey,SessionToken,Expiration]",
			"--output",
			"text",
		])
		.run_async_stdout()
		.await
		.map_err(|err| {
			bevyhow!(
				"could not assume `{role}` in `{}` ({}): {err}",
				self.name,
				self.id
			)
		})?;
		AdminSession::parse(&body, Self::SESSION_SECS)
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A name is quoted into the lookup, so anything but a repo's name is
	/// refused before it reaches the api.
	#[beet_core::test]
	fn checks_the_name() {
		MemberAccount::check_name("beetmash").unwrap();
		MemberAccount::check_name("beet_egress").unwrap();
		MemberAccount::check_name("a' || b").unwrap_err();
		MemberAccount::check_name("").unwrap_err();
	}
}

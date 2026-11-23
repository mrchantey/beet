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

	/// The Organizations api as `credentials`, answered by the management
	/// account from `us-east-1`.
	pub fn organizations(
		credentials: impl Into<AwsCredentialSource>,
	) -> AwsClient {
		AwsClient::new(AwsService::ORGANIZATIONS, "us-east-1")
			.with_credentials(credentials)
	}

	/// The active member account named `name`, asked as `credentials`: the
	/// administrator session, or `beet-agent` through its read policy.
	pub async fn find(
		credentials: impl Into<AwsCredentialSource>,
		name: &str,
	) -> Result<Self> {
		Self::find_in(&Self::organizations(credentials), name).await
	}

	/// [`find`](Self::find) through `organizations`.
	pub async fn find_in(
		organizations: &AwsClient,
		name: &str,
	) -> Result<Self> {
		Self::lookup_in(organizations, name).await?.ok_or_else(|| {
			bevyhow!(
				"the Organization holds no active account named `{name}`: an \
				elevated deploy of the package declaring `<AwsAccount \
				name=\"{name}\"/>` creates it"
			)
		})
	}

	/// [`find`](Self::find), `None` when there is no such account.
	pub async fn lookup(
		credentials: impl Into<AwsCredentialSource>,
		name: &str,
	) -> Result<Option<Self>> {
		Self::lookup_in(&Self::organizations(credentials), name).await
	}

	/// [`lookup`](Self::lookup) through `organizations`: every page of
	/// `ListAccounts`, the active ones named `name`.
	pub async fn lookup_in(
		organizations: &AwsClient,
		name: &str,
	) -> Result<Option<Self>> {
		Self::check_name(name)?;
		let mut ids = Vec::new();
		let mut next_token: Option<String> = None;
		loop {
			let body = match next_token.take() {
				Some(token) => serde_json::json!({"NextToken": token}),
				None => serde_json::json!({}),
			};
			let page = organizations.call_json("ListAccounts", &body).await?;
			ids.extend(
				page["Accounts"]
					.as_array()
					.into_iter()
					.flatten()
					.filter(|account| {
						account["Name"] == name
							&& (account["State"] == "ACTIVE"
								|| account["Status"] == "ACTIVE")
					})
					.filter_map(|account| account["Id"].as_str())
					.map(SmolStr::new),
			);
			match page["NextToken"].as_str() {
				Some(token) => next_token = Some(token.to_string()),
				None => break,
			}
		}
		match ids.as_slice() {
			[] => None.xok(),
			[id] => Some(Self {
				name: name.into(),
				id: id.clone(),
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

	/// Assume `role` in this account as `credentials`, for the hour a member
	/// session lasts.
	pub async fn assume(
		&self,
		credentials: impl Into<AwsCredentialSource>,
		role: &str,
	) -> Result<AwsCredentials> {
		self.assume_with(&AwsSts::new(credentials), role).await
	}

	/// [`assume`](Self::assume) through `sts`.
	pub async fn assume_with(
		&self,
		sts: &AwsSts,
		role: &str,
	) -> Result<AwsCredentials> {
		sts.assume_role(&AssumeRole {
			role_arn: format!("arn:aws:iam::{}:role/{role}", self.id).into(),
			session_name: AgentIdentity::USER.into(),
			duration_secs: Self::SESSION_SECS,
			mfa: None,
		})
		.await
		.map_err(|err| {
			bevyhow!(
				"could not assume `{role}` in `{}` ({}): {err}",
				self.name,
				self.id
			)
		})
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

	/// The account is found across pages by name and state, and the closed
	/// one of the same name is not it.
	#[beet_core::test]
	async fn finds_across_pages() {
		let pages = std::sync::Arc::new(std::sync::Mutex::new(
			vec![
				r#"{"Accounts":[{"Id":"111","Name":"beetmash","State":"SUSPENDED"}],"NextToken":"t1"}"#,
				r#"{"Accounts":[{"Id":"222","Name":"beetmash","State":"ACTIVE"},{"Id":"333","Name":"other","State":"ACTIVE"}]}"#,
			]
			.into_iter(),
		));
		let tokens = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
		let seen = tokens.clone();
		let organizations =
			AwsClient::new(AwsService::ORGANIZATIONS, "us-east-1")
				.with_credentials(AwsCredentials::new("AKID", "secret"))
				.with_transport(move |request| {
					let pages = pages.clone();
					let seen = seen.clone();
					Box::pin(async move {
						request.headers.first_raw("x-amz-target").xpect_eq(
							Some("AWSOrganizationsV20161128.ListAccounts"),
						);
						let body = request.body.into_bytes().await?;
						seen.lock()
							.unwrap()
							.push(String::from_utf8(body.to_vec())?);
						Response::ok()
							.with_body(pages.lock().unwrap().next().unwrap())
							.xok()
					})
				});
		MemberAccount::find_in(&organizations, "beetmash")
			.await
			.unwrap()
			.id
			.as_str()
			.xpect_eq("222");
		tokens.lock().unwrap().clone().xpect_eq(vec![
			"{}".to_string(),
			r#"{"NextToken":"t1"}"#.to_string(),
		]);
	}
}

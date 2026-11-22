//! `<IdentityCenter/>`: the IAM Identity Center instance a package's people
//! sign in through, and the administration calls that provision it.
#[allow(unused_imports)]
use crate::prelude::*;
use beet_core::prelude::*;
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
use beet_net::prelude::*;
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
use serde_json::Value;
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
use serde_json::json;

/// The IAM Identity Center instance a package's member accounts are reached
/// through by a person, ie `{IdentityCenter{region:"us-east-1"}}` beside the
/// `<AwsAccount/>` on the package root, resolved by ancestry like
/// [`AwsRegion`].
///
/// The account converge provisions what an api can (the operator group, the
/// two permission sets, each member's assignments); enabling the instance
/// is the console's, see [`AwsAccount`]'s docs. A person then signs in once,
/// `beet sso login`, and reaches every assigned account from the terminal
/// and the access portal alike.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct IdentityCenter {
	/// The region the instance lives in, else the nearest [`AwsRegion`].
	pub region: Option<SmolStr>,
	/// The group every operator belongs to, assigned to every member account.
	pub group: SmolStr,
	/// The access portal's url where the instance has an alias
	/// (`https://<alias>.awsapps.com/start`), else derived from its identity
	/// store id.
	pub start_url: Option<SmolStr>,
}

impl Default for IdentityCenter {
	fn default() -> Self {
		Self {
			region: None,
			group: "beet-operators".into(),
			start_url: None,
		}
	}
}

#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
/// One Identity Center instance as `ListInstances` answers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityCenterInstance {
	/// The instance's arn, which every administration call names.
	pub arn: SmolStr,
	/// Its identity store, where users and groups live.
	pub identity_store_id: SmolStr,
}

#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
impl IdentityCenterInstance {
	/// The access portal's url, derived from the identity store id.
	pub fn start_url(&self) -> String {
		format!("https://{}.awsapps.com/start", self.identity_store_id)
	}
}

#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
/// A permission set the converge keeps, the managed policy it carries and
/// how long its sessions last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionSetSpec {
	/// Its name, which is also the role name the portal mints credentials
	/// for.
	pub name: &'static str,
	/// The AWS managed policy it carries.
	pub policy_arn: &'static str,
	/// Its session duration, ISO 8601.
	pub session: &'static str,
	/// Its description.
	pub description: &'static str,
}

#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
impl IdentityCenter {
	/// The two permission sets, mirroring the role split: write behind a
	/// fresh sign in, read for a working day.
	pub const PERMISSION_SETS: [PermissionSetSpec; 2] = [
		PermissionSetSpec {
			name: IdentityCenterSession::ADMIN_SET,
			policy_arn: "arn:aws:iam::aws:policy/AdministratorAccess",
			session: "PT1H",
			description: "beet: administer a member account, for an hour",
		},
		PermissionSetSpec {
			name: IdentityCenterSession::READ_SET,
			policy_arn: "arn:aws:iam::aws:policy/job-function/ViewOnlyAccess",
			session: "PT8H",
			description: "beet: read a member account, for a working day",
		},
	];

	/// The declared instance and its region, `None` when this launch
	/// declares none.
	pub async fn declared(
		caller: &AsyncEntity,
	) -> Result<Option<(Self, SmolStr)>> {
		caller
			.with_state::<(Query<(Entity, &IdentityCenter)>, AncestorQuery<&AwsRegion>), _>(
				|_, (centers, regions)| {
					let mut found = centers.iter();
					let Some((entity, center)) = found.next() else {
						return None.xok();
					};
					if found.next().is_some() {
						bevybail!(
							"this launch declares two `IdentityCenter`s, and an \
							Organization has one: keep one declaration"
						);
					}
					let region = match &center.region {
						Some(region) => region.clone(),
						None => regions
							.get(entity)
							.map(|region| region.0.clone())
							.map_err(|_| {
								bevyhow!(
									"`IdentityCenter` names no region and has no \
									`AwsRegion` on it or an ancestor"
								)
							})?,
					};
					Some((center.clone(), region)).xok()
				},
			)
			.await?
	}

	/// The declared instance, refused when none is declared.
	pub async fn require(caller: &AsyncEntity) -> Result<(Self, SmolStr)> {
		Self::declared(caller).await?.ok_or_else(|| {
			bevyhow!(
				"this launch declares no `IdentityCenter`: spread \
				`{{IdentityCenter{{region:\"us-east-1\"}}}}` beside the package's \
				`<AwsAccount/>`"
			)
		})
	}
}

#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
/// The Identity Center administration api as one credential: the instance
/// (`sso-admin`) and its identity store, answered by the management account.
#[derive(Debug, Clone)]
pub struct IdentityCenterAdmin {
	sso_admin: AwsClient,
	identity_store: AwsClient,
}

#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
impl IdentityCenterAdmin {
	/// The administration api in `region` as `credentials`.
	pub fn new(
		region: &str,
		credentials: impl Into<AwsCredentialSource>,
	) -> Self {
		let credentials = credentials.into();
		Self {
			sso_admin: AwsClient::new(AwsService::SSO_ADMIN, region)
				.with_credentials(credentials.clone()),
			identity_store: AwsClient::new(AwsService::IDENTITY_STORE, region)
				.with_credentials(credentials),
		}
	}

	/// The api through `sso_admin` and `identity_store`, clients a caller
	/// prepared (ie with its own test seam).
	pub fn from_clients(
		sso_admin: AwsClient,
		identity_store: AwsClient,
	) -> Self {
		Self {
			sso_admin,
			identity_store,
		}
	}

	/// The same api through `transport`, a test's stand-in.
	pub fn with_transport(
		mut self,
		transport: impl 'static
		+ Send
		+ Sync
		+ Clone
		+ Fn(Request) -> SendBoxedFuture<Result<Response>>,
	) -> Self {
		self.sso_admin = self.sso_admin.with_transport(transport.clone());
		self.identity_store = self.identity_store.with_transport(transport);
		self
	}

	/// The Organization's instance, `None` while it is not enabled.
	pub async fn instance(&self) -> Result<Option<IdentityCenterInstance>> {
		let answer = self
			.sso_admin
			.call_json("ListInstances", &json!({}))
			.await?;
		answer["Instances"]
			.as_array()
			.into_iter()
			.flatten()
			.find(|instance| {
				instance["Status"]
					.as_str()
					.is_none_or(|status| status == "ACTIVE")
			})
			.and_then(|instance| {
				IdentityCenterInstance {
					arn: instance["InstanceArn"].as_str()?.into(),
					identity_store_id: instance["IdentityStoreId"]
						.as_str()?
						.into(),
				}
				.xsome()
			})
			.xok()
	}

	/// `identifier` (an attribute path and value) resolved to its id through
	/// `action`, `None` when the store holds no such entity.
	async fn find_id(
		&self,
		instance: &IdentityCenterInstance,
		action: &str,
		path: &str,
		value: &str,
		id_field: &str,
	) -> Result<Option<SmolStr>> {
		match self
			.identity_store
			.call_json(
				action,
				&json!({
					"IdentityStoreId": instance.identity_store_id.as_str(),
					"AlternateIdentifier": {"UniqueAttribute": {
						"AttributePath": path,
						"AttributeValue": value,
					}},
				}),
			)
			.await
		{
			Ok(answer) => answer[id_field].as_str().map(SmolStr::new).xok(),
			Err(err) if AwsError::is(&err, &["ResourceNotFoundException"]) => {
				None.xok()
			}
			Err(err) => Err(err),
		}
	}

	/// The group named `name`.
	pub async fn find_group(
		&self,
		instance: &IdentityCenterInstance,
		name: &str,
	) -> Result<Option<SmolStr>> {
		self.find_id(instance, "GetGroupId", "displayName", name, "GroupId")
			.await
	}

	/// Create the group `name`, answering its id.
	pub async fn create_group(
		&self,
		instance: &IdentityCenterInstance,
		name: &str,
	) -> Result<SmolStr> {
		self.identity_store
			.call_json(
				"CreateGroup",
				&json!({
					"IdentityStoreId": instance.identity_store_id.as_str(),
					"DisplayName": name,
					"Description": "beet: the operators, assigned to every member account",
				}),
			)
			.await?["GroupId"]
			.as_str()
			.map(SmolStr::new)
			.ok_or_else(|| bevyhow!("CreateGroup answered no group id"))
	}

	/// The user named `user_name`.
	pub async fn find_user(
		&self,
		instance: &IdentityCenterInstance,
		user_name: &str,
	) -> Result<Option<SmolStr>> {
		self.find_id(instance, "GetUserId", "userName", user_name, "UserId")
			.await
	}

	/// Create the user `email` (its user name too), answering its id.
	pub async fn create_user(
		&self,
		instance: &IdentityCenterInstance,
		email: &str,
		given: &str,
		family: &str,
	) -> Result<SmolStr> {
		self.identity_store
			.call_json(
				"CreateUser",
				&json!({
					"IdentityStoreId": instance.identity_store_id.as_str(),
					"UserName": email,
					"DisplayName": format!("{given} {family}").trim(),
					"Name": {"GivenName": given, "FamilyName": family},
					"Emails": [{"Value": email, "Type": "work", "Primary": true}],
				}),
			)
			.await?["UserId"]
			.as_str()
			.map(SmolStr::new)
			.ok_or_else(|| bevyhow!("CreateUser answered no user id"))
	}

	/// Add `user` to `group` unless it is already a member, answering
	/// whether it was added.
	pub async fn ensure_member(
		&self,
		instance: &IdentityCenterInstance,
		group: &str,
		user: &str,
	) -> Result<bool> {
		let body = json!({
			"IdentityStoreId": instance.identity_store_id.as_str(),
			"GroupId": group,
			"MemberId": {"UserId": user},
		});
		match self
			.identity_store
			.call_json("GetGroupMembershipId", &body)
			.await
		{
			Ok(_) => return false.xok(),
			Err(err) if AwsError::is(&err, &["ResourceNotFoundException"]) => {}
			Err(err) => return Err(err),
		}
		self.identity_store
			.call_json("CreateGroupMembership", &body)
			.await?;
		true.xok()
	}

	/// The permission set named `name`, by arn.
	pub async fn find_permission_set(
		&self,
		instance: &IdentityCenterInstance,
		name: &str,
	) -> Result<Option<SmolStr>> {
		let mut next: Option<String> = None;
		loop {
			let mut body = json!({"InstanceArn": instance.arn.as_str()});
			if let Some(token) = next.take() {
				body["NextToken"] = json!(token);
			}
			let page = self
				.sso_admin
				.call_json("ListPermissionSets", &body)
				.await?;
			for arn in page["PermissionSets"]
				.as_array()
				.into_iter()
				.flatten()
				.filter_map(Value::as_str)
			{
				let described = self
					.sso_admin
					.call_json(
						"DescribePermissionSet",
						&json!({
							"InstanceArn": instance.arn.as_str(),
							"PermissionSetArn": arn,
						}),
					)
					.await?;
				if described["PermissionSet"]["Name"] == name {
					return Some(SmolStr::new(arn)).xok();
				}
			}
			match page["NextToken"].as_str() {
				Some(token) => next = Some(token.to_string()),
				None => return None.xok(),
			}
		}
	}

	/// Create the permission set `spec`, answering its arn.
	pub async fn create_permission_set(
		&self,
		instance: &IdentityCenterInstance,
		spec: &PermissionSetSpec,
	) -> Result<SmolStr> {
		self.sso_admin
			.call_json(
				"CreatePermissionSet",
				&json!({
					"InstanceArn": instance.arn.as_str(),
					"Name": spec.name,
					"Description": spec.description,
					"SessionDuration": spec.session,
				}),
			)
			.await?["PermissionSet"]["PermissionSetArn"]
			.as_str()
			.map(SmolStr::new)
			.ok_or_else(|| bevyhow!("CreatePermissionSet answered no arn"))
	}

	/// Attach `policy` to the permission set unless it holds it, answering
	/// whether it was attached.
	pub async fn ensure_policy(
		&self,
		instance: &IdentityCenterInstance,
		set: &str,
		policy: &str,
	) -> Result<bool> {
		let held = self
			.sso_admin
			.call_json(
				"ListManagedPoliciesInPermissionSet",
				&json!({
					"InstanceArn": instance.arn.as_str(),
					"PermissionSetArn": set,
				}),
			)
			.await?;
		if held["AttachedManagedPolicies"]
			.as_array()
			.into_iter()
			.flatten()
			.any(|attached| attached["Arn"] == policy)
		{
			return false.xok();
		}
		self.sso_admin
			.call_json(
				"AttachManagedPolicyToPermissionSet",
				&json!({
					"InstanceArn": instance.arn.as_str(),
					"PermissionSetArn": set,
					"ManagedPolicyArn": policy,
				}),
			)
			.await?;
		true.xok()
	}

	/// Provision the permission set into every account it is assigned in,
	/// polling until it settles.
	pub async fn provision(
		&self,
		instance: &IdentityCenterInstance,
		set: &str,
	) -> Result {
		let started = self
			.sso_admin
			.call_json(
				"ProvisionPermissionSet",
				&json!({
					"InstanceArn": instance.arn.as_str(),
					"PermissionSetArn": set,
					"TargetType": "ALL_PROVISIONED_ACCOUNTS",
				}),
			)
			.await?;
		let status = &started["PermissionSetProvisioningStatus"];
		let Some(request) = status["RequestId"].as_str() else {
			bevybail!("ProvisionPermissionSet answered no request id");
		};
		self.settle(
			status.clone(),
			"DescribePermissionSetProvisioningStatus",
			json!({
				"InstanceArn": instance.arn.as_str(),
				"ProvisionPermissionSetRequestId": request,
			}),
			"PermissionSetProvisioningStatus",
		)
		.await
	}

	/// Whether the group holds the permission set in `account`.
	pub async fn is_assigned(
		&self,
		instance: &IdentityCenterInstance,
		account: &str,
		set: &str,
		group: &str,
	) -> Result<bool> {
		let mut next: Option<String> = None;
		loop {
			let mut body = json!({
				"InstanceArn": instance.arn.as_str(),
				"AccountId": account,
				"PermissionSetArn": set,
			});
			if let Some(token) = next.take() {
				body["NextToken"] = json!(token);
			}
			let page = self
				.sso_admin
				.call_json("ListAccountAssignments", &body)
				.await?;
			if page["AccountAssignments"]
				.as_array()
				.into_iter()
				.flatten()
				.any(|assignment| {
					assignment["PrincipalType"] == "GROUP"
						&& assignment["PrincipalId"] == group
				}) {
				return true.xok();
			}
			match page["NextToken"].as_str() {
				Some(token) => next = Some(token.to_string()),
				None => return false.xok(),
			}
		}
	}

	/// Assign the group the permission set in `account`, or with `remove`
	/// take the assignment away, polling until it settles.
	pub async fn assign(
		&self,
		instance: &IdentityCenterInstance,
		account: &str,
		set: &str,
		group: &str,
		remove: bool,
	) -> Result {
		let (action, status_field, describe, request_field) = match remove {
			false => (
				"CreateAccountAssignment",
				"AccountAssignmentCreationStatus",
				"DescribeAccountAssignmentCreationStatus",
				"AccountAssignmentCreationRequestId",
			),
			true => (
				"DeleteAccountAssignment",
				"AccountAssignmentDeletionStatus",
				"DescribeAccountAssignmentDeletionStatus",
				"AccountAssignmentDeletionRequestId",
			),
		};
		let started = self
			.sso_admin
			.call_json(
				action,
				&json!({
					"InstanceArn": instance.arn.as_str(),
					"TargetId": account,
					"TargetType": "AWS_ACCOUNT",
					"PermissionSetArn": set,
					"PrincipalType": "GROUP",
					"PrincipalId": group,
				}),
			)
			.await?;
		let status = &started[status_field];
		let Some(request) = status["RequestId"].as_str() else {
			bevybail!("{action} answered no request id");
		};
		let mut query = json!({"InstanceArn": instance.arn.as_str()});
		query[request_field] = json!(request);
		self.settle(status.clone(), describe, query, status_field)
			.await
	}

	/// Poll `describe` with `query` until the status at `field` leaves
	/// `IN_PROGRESS`, from the first answer `status`.
	async fn settle(
		&self,
		mut status: Value,
		describe: &str,
		query: Value,
		field: &str,
	) -> Result {
		for _ in 0..60 {
			match status["Status"].as_str().unwrap_or_default() {
				"SUCCEEDED" => return OK,
				"FAILED" => bevybail!(
					"{describe} failed: {}",
					status["FailureReason"]
						.as_str()
						.unwrap_or("no reason given")
				),
				_ => {
					time_ext::sleep_secs(2).await;
					status = self.sso_admin.call_json(describe, &query).await?
						[field]
						.clone();
				}
			}
		}
		bevybail!("{describe} had not settled after two minutes")
	}
}

#[cfg(all(test, feature = "deploy", not(target_arch = "wasm32")))]
mod test {
	use super::*;
	use std::sync::Arc;
	use std::sync::Mutex;

	/// An api answering each target from `answers`, keeping every target it
	/// was asked.
	fn admin(
		answers: &'static [(&'static str, &'static str)],
	) -> (IdentityCenterAdmin, Arc<Mutex<Vec<String>>>) {
		let asked = Arc::new(Mutex::new(Vec::new()));
		let seen = asked.clone();
		let admin = IdentityCenterAdmin::new(
			"us-east-1",
			AwsCredentials::new("AKID", "secret"),
		)
		.with_transport(move |request: Request| {
			let seen = seen.clone();
			Box::pin(async move {
				let target = request
					.headers
					.first_raw("x-amz-target")
					.unwrap_or_default()
					.rsplit('.')
					.next()
					.unwrap_or_default()
					.to_string();
				seen.lock().unwrap().push(target.clone());
				match answers.iter().find(|(name, _)| *name == target) {
					Some((_, body)) => Response::ok().with_body(*body).xok(),
					None => Response::from_status(StatusCode::BAD_REQUEST)
						.with_body(
							r#"{"__type":"ResourceNotFoundException","message":"none"}"#,
						)
						.xok(),
				}
			})
		});
		(admin, asked)
	}

	fn instance() -> IdentityCenterInstance {
		IdentityCenterInstance {
			arn: "arn:aws:sso:::instance/ssoins-1".into(),
			identity_store_id: "d-123".into(),
		}
	}

	/// An Organization with no instance answers none; an active one its arn,
	/// store and start url.
	#[beet_core::test]
	async fn finds_the_instance() {
		admin(&[("ListInstances", r#"{"Instances":[]}"#)])
			.0
			.instance()
			.await
			.unwrap()
			.xpect_none();
		let found = admin(&[(
			"ListInstances",
			r#"{"Instances":[{"InstanceArn":"arn:aws:sso:::instance/ssoins-1","IdentityStoreId":"d-123","Status":"ACTIVE"}]}"#,
		)])
		.0
		.instance()
		.await
		.unwrap()
		.unwrap();
		found
			.start_url()
			.xpect_eq("https://d-123.awsapps.com/start");
	}

	/// A missing group is none, then created; a membership already held is
	/// left.
	#[beet_core::test]
	async fn finds_else_creates() {
		let (admin, asked) = admin(&[
			(
				"CreateGroup",
				r#"{"GroupId":"g-1","IdentityStoreId":"d-123"}"#,
			),
			(
				"GetGroupMembershipId",
				r#"{"MembershipId":"m-1","IdentityStoreId":"d-123"}"#,
			),
		]);
		admin
			.find_group(&instance(), "beet-operators")
			.await
			.unwrap()
			.xpect_none();
		admin
			.create_group(&instance(), "beet-operators")
			.await
			.unwrap()
			.as_str()
			.xpect_eq("g-1");
		admin
			.ensure_member(&instance(), "g-1", "u-1")
			.await
			.unwrap()
			.xpect_false();
		asked.lock().unwrap().clone().xpect_eq(vec![
			"GetGroupId".to_string(),
			"CreateGroup".into(),
			"GetGroupMembershipId".into(),
		]);
	}

	/// An assignment settles through its status poll.
	#[beet_core::test]
	async fn assigns_and_settles() {
		let (admin, asked) = admin(&[(
			"CreateAccountAssignment",
			r#"{"AccountAssignmentCreationStatus":{"Status":"SUCCEEDED","RequestId":"r-1"}}"#,
		)]);
		admin
			.assign(&instance(), "222", "arn:ps", "g-1", false)
			.await
			.unwrap();
		asked
			.lock()
			.unwrap()
			.clone()
			.xpect_eq(vec!["CreateAccountAssignment".to_string()]);
	}
}

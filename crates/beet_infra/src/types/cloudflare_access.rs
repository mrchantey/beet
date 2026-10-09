//! What a Cloudflare action asks of the deploy token, declared on the action.

#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
use crate::actions::cloudflare_api_ext;
use crate::prelude::*;
use beet_core::prelude::*;
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
use beet_net::prelude::Request;

/// What a Cloudflare action asks of the deploy token, declared on the action
/// itself, and the only way that action reaches the token.
///
/// An action that calls Cloudflare requires one naming the groups its calls
/// need, ie `#[require(CloudflareAccess = CloudflareAccess::new::<Self>(&[TokenPermission::CACHE_PURGE]))]`,
/// so the declaration sits beside the calls it describes. The deploy
/// credential ([`CloudflareDeployToken`]) lowers every one in the world into
/// the repo's deploy token
/// ([`DeployerToken::lower_access`]), and the action holds that token only
/// through [`resolve`](Self::resolve) on its own entity.
///
/// The two halves are what keep the declaration honest. A table of action
/// names kept anywhere else drifts silently, since an action missing from it
/// is simply never lowered and the minted token answers 403 at the first
/// deploy that runs it, which is how the MTA-STS publish once went short. Here
/// an action that calls Cloudflare without declaring what for fails at its
/// first call, naming the fix. An action that reaches nothing at Cloudflare
/// declares nothing, and can reach nothing.
///
/// A bucket-scoped group (an object read or write over the S3 api) is granted
/// over one bucket, which the declaration cannot know statically, so it names
/// where the action keeps it: [`with_bucket`](Self::with_bucket), read off
/// the action's own entity when the token is lowered.
///
/// The converge of the deploy credential is the one reader of the token that
/// does not go through this: it runs as the mint token, which no action
/// lowers.
#[derive(Debug, Clone, Copy, Component)]
pub struct CloudflareAccess {
	/// The action's type path, reported as what asked for each group.
	action: &'static str,
	/// The groups the action's calls need, from the one vocabulary
	/// [`TokenPermission`] holds.
	permissions: &'static [TokenPermission],
	/// The bucket a bucket-scoped group is granted over, read off the action's
	/// own entity, ie its `bucket` field.
	bucket: Option<fn(EntityRef) -> Option<SmolStr>>,
}

/// Two declarations are the same when they ask the same groups for the same
/// action: the bucket is where a value is read from, not a value.
impl PartialEq for CloudflareAccess {
	fn eq(&self, other: &Self) -> bool {
		self.action == other.action && self.permissions == other.permissions
	}
}

impl Eq for CloudflareAccess {}

impl CloudflareAccess {
	/// The declaration of the action `T`, ie `CloudflareAccess::new::<Self>`
	/// inside the action's own `#[require]`.
	pub fn new<T: 'static>(permissions: &'static [TokenPermission]) -> Self {
		Self {
			action: core::any::type_name::<T>(),
			permissions,
			bucket: None,
		}
	}

	/// Where the action keeps the bucket its bucket-scoped groups are granted
	/// over, ie `|entity| entity.get::<Self>().map(|sync| sync.bucket.clone())`
	/// inside the action's own `#[require]`.
	pub const fn with_bucket(
		mut self,
		bucket: fn(EntityRef) -> Option<SmolStr>,
	) -> Self {
		self.bucket = Some(bucket);
		self
	}

	/// The bucket `entity`, the action declaring this, names for its
	/// bucket-scoped groups, `None` when it names none.
	pub fn bucket(&self, entity: EntityRef) -> Option<SmolStr> {
		self.bucket.and_then(|bucket| bucket(entity))
	}

	/// The action's short name, ie `MtaStsPublish`.
	pub fn action(&self) -> &'static str {
		self.action.rsplit("::").next().unwrap_or(self.action)
	}

	/// The groups the action's calls need.
	pub fn permissions(&self) -> &'static [TokenPermission] { self.permissions }

	/// The declaration on `caller`, the entity running the action: the
	/// capability every deploy call goes through.
	pub async fn resolve(caller: &AsyncEntity) -> Result<Self> {
		caller.with(|entity| Self::of(entity.as_readonly())).await?
	}

	/// The declaration on `entity`, an error naming the fix when it has none.
	pub fn of(entity: EntityRef) -> Result<Self> {
		entity.get::<Self>().copied().ok_or_else(|| {
			bevyhow!(
				"entity {} reaches Cloudflare but declares no `CloudflareAccess`: \
				require one on the action naming the groups its calls need, so \
				the elevated deploy lowers them into the deploy token",
				entity.id()
			)
		})
	}
}

/// The calls the declaration gates, compiled with the actions that make them.
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
impl CloudflareAccess {
	/// The deploy token from the environment, the bearer of every api call an
	/// action makes.
	pub fn token(&self) -> Result<SmolStr> { cloudflare_api_ext::token() }

	/// The zone id `caller` resolves by ancestry
	/// ([`ResolvedStack::cloudflare_zone`]) and the token beside it, the auth
	/// every zone-level call needs.
	pub async fn zone_auth(
		&self,
		caller: &AsyncEntity,
	) -> Result<(SmolStr, SmolStr)> {
		let zone_id = caller
			.with_state::<StackQuery, _>(|entity, stacks| {
				stacks
					.resolve(entity)
					.cloudflare_zone()
					.map(|zone| zone.id.clone())
			})
			.await??;
		Ok((zone_id, self.token()?))
	}

	/// A `wrangler` child carrying the deploy token, redacted from its output.
	/// With no token in the environment the child is left to wrangler's own
	/// resolution, so a credential-free run still writes its project.
	pub fn wrangler(&self) -> ChildProcess {
		let child = ChildProcess::new("wrangler");
		match self.token() {
			Ok(token) => child
				.with_envs([("CLOUDFLARE_API_TOKEN", token.as_str())])
				.with_secret(token),
			Err(_) => child,
		}
	}

	/// The S3 credentials for R2 derived from the deploy token, in `account`:
	/// a token's id IS the access key id and the lowercase hex SHA-256 of its
	/// value IS the secret access key
	/// ([`cloudflare_api_ext::derive_secret_key`]). So an R2 data-plane
	/// credential is a DERIVATION of the one credential a Cloudflare command
	/// already carries, never a second credential anybody holds: a hand-made
	/// pair with object write over every bucket in the account is final in its
	/// damage (R2 keeps no versions), and a document that sealed one would hold
	/// a credential the age key must not open.
	///
	/// The id is read back with the token's own verify, the one call a token
	/// may always make about itself. Account-owned, which is what the account's
	/// api-token pages mint (the R2 page included) and the only kind that
	/// verify answers for.
	pub async fn r2_credentials(
		&self,
		account: &str,
	) -> Result<(String, String)> {
		let token = self.token()?;
		let verified = cloudflare_api_ext::send(
			Request::get(format!(
				"{}/accounts/{account}/tokens/verify",
				cloudflare_api_ext::API_BASE
			))
			.with_auth_bearer(&token),
			"verifying the api token to derive its R2 S3 pair (account-owned, \
			which a token made under My Profile is not)",
		)
		.await?;
		match verified["result"]["id"].as_str() {
			Some(id) => (
				id.to_string(),
				cloudflare_api_ext::derive_secret_key(&token),
			)
				.xok(),
			None => bevybail!(
				"the api token's verify answered no id, so no R2 S3 pair could \
				be derived"
			),
		}
	}
}

// its callers are the deploy actions' tests, native only
#[cfg(all(test, feature = "deploy", not(target_arch = "wasm32")))]
impl CloudflareAccess {
	/// The declaration the action `T` requires, read off a spawned `T` the way
	/// the mint reads one off a scene, so a test pins what the action really
	/// declares rather than a copy of it.
	pub(crate) fn declared_by<T: Component + Default>() -> Self {
		let mut world = World::new();
		let entity = world.spawn(T::default()).id();
		Self::of(world.entity(entity)).unwrap()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	struct PurgeLike;

	/// The declaration names its action by the short type name, which is what a
	/// lowering reports as asking for each group.
	#[beet_core::test]
	fn names_the_action_it_declares() {
		let access =
			CloudflareAccess::new::<PurgeLike>(&[TokenPermission::CACHE_PURGE]);
		access.action().xpect_eq("PurgeLike");
		access
			.permissions()
			.to_vec()
			.xpect_eq(vec![TokenPermission::CACHE_PURGE]);
	}

	/// An entity that reaches for the token without declaring what for is
	/// refused, naming the fix: the guard that keeps every Cloudflare call
	/// lowered, since an undeclared action would otherwise be invisible to the
	/// mint and 403 at the first deploy that runs it.
	#[beet_core::test]
	fn an_undeclared_caller_is_refused() {
		let mut world = World::new();
		let undeclared = world.spawn_empty().id();
		CloudflareAccess::of(world.entity(undeclared))
			.unwrap_err()
			.to_string()
			.xpect_contains("declares no `CloudflareAccess`")
			.xpect_contains("lowers them into the deploy token");
		let declared = world
			.spawn(CloudflareAccess::new::<PurgeLike>(&[
				TokenPermission::CACHE_PURGE,
			]))
			.id();
		CloudflareAccess::of(world.entity(declared))
			.unwrap()
			.action()
			.xpect_eq("PurgeLike");
	}
}

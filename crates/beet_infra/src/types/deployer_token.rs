//! The Cloudflare api token a repo's deployer needs, LOWERED from the provider
//! types its stacks render and the Cloudflare actions their routes run.

use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::Value;
use serde_json::json;

/// The Cloudflare counterpart of [`DeployerPolicy`]: that lowers the AWS types
/// a stack renders into what the deployer applying it may do, this lowers the
/// CLOUDFLARE ones, plus the Cloudflare actions a deploy route runs, which
/// reach the api without rendering anything at all.
///
/// ## Why the actions are a second input
///
/// On the AWS side every call a deploy makes belongs to a service that some
/// rendered resource names, so the config is the whole input. Cloudflare's
/// zone work is not like that: `<CloudflareZoneSetup/>` publishes a ruleset and
/// patches two settings, `<CloudflarePurgeCache/>` purges, `<ZoneAudit/>` lists
/// and deletes records, and none of the three renders a resource. A token
/// lowered from the config alone would be three groups short on every deploy,
/// so each action declares its own on itself ([`CloudflareAccess`]), and that
/// declaration is also the only way the action reaches the token, so an action
/// cannot call Cloudflare without being lowered.
///
/// ## What Cloudflare does not have, and what this is instead
///
/// There is no permissions boundary and no mfa condition on an api token, so
/// neither half of the AWS ceiling transfers: a token's power is exactly the
/// groups and resources it was created with, for as long as it exists. So this
/// lowering IS the ceiling rather than a derivation of one, and the one group
/// that escalates ([`TokenPermission::API_TOKENS_WRITE`], which may mint any
/// token the account can hold, a wider one included) is a line the credential
/// model has to draw by hand. [`escalating`](Self::escalating) names who asks
/// for it.
///
/// ## What makes a token out of this
///
/// `cloudflare/mint` ([`CloudflareMint`]) posts [`to_json`](Self::to_json) as
/// an account-owned token and seals it as `CLOUDFLARE_API_TOKEN`; [`Display`]
/// prints the same lowering for its `--dry-run`. Each group carries the api's
/// own id beside its name, so the mint needs no second table.
///
/// [`Display`]: std::fmt::Display
#[derive(Debug, Default, Clone)]
pub struct DeployerToken {
	/// The accounts the lowered stacks address: where the token lives, and the
	/// `Account Resources` of an account-scoped group. Recorded for a zone-only
	/// lowering too, since an account-owned token is created under an account
	/// however narrow its groups are; [`account`](Self::account) is the one a
	/// mint uses, and refuses to guess between two.
	accounts: BTreeSet<SmolStr>,
	/// The zones the lowered stacks address by domain, the `Zone Resources` of
	/// every zone-scoped group. Keyed by domain because that is what the
	/// dashboard's zone picker shows, with the id a mint would use.
	zones: BTreeMap<SmolStr, SmolStr>,
	/// Every permission the lowering named, and the declared types and actions
	/// that asked for each: a narrowing has to say which declaration to remove,
	/// not only which group to drop.
	asked: BTreeMap<TokenPermission, BTreeSet<SmolStr>>,
}

impl DeployerToken {
	/// The one declared-type prefix this lowers. Every other provider's types
	/// lower through that provider's own policy, so they are skipped rather
	/// than refused.
	const PREFIX: &'static str = "cloudflare_";

	/// What a rendered provider type needs, by longest matching prefix so a
	/// family rides one entry, and loud on a `cloudflare_` type with no entry
	/// at all ([`DeployerPolicy::service`]'s rule, for the same reason: a
	/// silently dropped type is a deploy that works until the apply that
	/// touches it).
	const RESOURCES: &'static [(&'static str, &'static [TokenPermission])] = &[
		// a tripwire rather than a need: nothing in either tree renders an api
		// token any more (`cloudflare/mint` mints out of band, see
		// `an_r2_bucket_never_escalates`), and a declaration that rendered one
		// again would name the escalating group out loud instead of quietly
		// widening every deploy credential that lowers it
		("cloudflare_account_token", &[
			TokenPermission::API_TOKENS_READ,
			TokenPermission::API_TOKENS_WRITE,
		]),
		("cloudflare_dns_record", &[TokenPermission::DNS_WRITE]),
		// a load balancer is zone-scoped, its monitors and pools account-scoped,
		// so the opt-in failover block asks for both lists
		("cloudflare_load_balancer", &[
			TokenPermission::LOAD_BALANCERS_WRITE,
		]),
		("cloudflare_load_balancer_monitor", &[
			TokenPermission::LOAD_BALANCER_POOLS_WRITE,
		]),
		("cloudflare_load_balancer_pool", &[
			TokenPermission::LOAD_BALANCER_POOLS_WRITE,
		]),
		("cloudflare_r2_bucket", &[
			TokenPermission::WORKERS_R2_STORAGE_WRITE,
		]),
	];

	/// Lower one of the repo's stacks: the groups its `cloudflare_` types need,
	/// and the account or zone each group's resource list names. A stack
	/// rendering no Cloudflare type at all adds nothing and demands no address.
	pub fn lower(
		mut self,
		stack: &ResolvedStack,
		config: &terra::Config,
	) -> Result<Self> {
		for declared in config
			.declared_types()
			.into_iter()
			.filter(|declared| declared.starts_with(Self::PREFIX))
		{
			self.add(stack, declared, Self::resource(declared)?)?;
		}
		self.xok()
	}

	/// Lower one Cloudflare action a route of the repo runs, by the
	/// [`CloudflareAccess`] it declares. The addresses come from the action's
	/// OWN stack, resolved by ancestry from its entity, since a route's verbs
	/// may sit outside every stack.
	pub fn lower_access(
		mut self,
		stack: &ResolvedStack,
		access: &CloudflareAccess,
	) -> Result<Self> {
		self.add(stack, access.action(), access.permissions())?;
		self.xok()
	}

	/// The permissions a declared type needs, an error for a `cloudflare_`
	/// type with no entry.
	fn resource(declared: &str) -> Result<&'static [TokenPermission]> {
		Self::RESOURCES
			.iter()
			.filter(|(prefix, _)| declared.starts_with(prefix))
			.max_by_key(|(prefix, _)| prefix.len())
			.map(|(_, permissions)| *permissions)
			.ok_or_else(|| {
				bevyhow!(
					"no deploy token permission is declared for `{declared}`: \
					add its prefix to `DeployerToken::RESOURCES`"
				)
			})
	}

	/// Record `permissions` as asked for by `asker`, resolving the addresses they
	/// need: the stack's [`CloudflareZone`] for a zone-scoped group, and its
	/// [`CloudflareAccount`] when it declares one.
	///
	/// A zone-scoped group needs the zone, so a stack without one fails here
	/// naming the spread. The ACCOUNT is noted rather than demanded, because
	/// only a mint needs it (a token lives in an account) and only a mint can
	/// say so usefully: [`account`](Self::account) is where the absence is an
	/// error, so a lowering still answers what an entry asks for when nobody
	/// has declared where its token would live.
	fn add(
		&mut self,
		stack: &ResolvedStack,
		asker: &str,
		permissions: &[TokenPermission],
	) -> Result {
		if permissions.is_empty() {
			return OK;
		}
		if let Ok(account) = stack.cloudflare_account() {
			self.accounts.insert(SmolStr::new(account.id()));
		}
		for permission in permissions {
			match permission.scope {
				TokenScope::Account => {}
				TokenScope::Zone => {
					let zone = stack.cloudflare_zone()?;
					self.zones.insert(zone.domain.clone(), zone.id.clone());
				}
				TokenScope::Bucket => bevybail!(
					"`{asker}` asks a deploy token for the bucket-scoped \
					`{permission}`, which belongs to a token an apply MINTS \
					rather than to the one applying it"
				),
			}
			self.asked
				.entry(*permission)
				.or_default()
				.insert(asker.into());
		}
		Ok(())
	}

	/// Every permission the lowering named, and what asked for each.
	pub fn asked(&self) -> &BTreeMap<TokenPermission, BTreeSet<SmolStr>> {
		&self.asked
	}

	/// The accounts every account-scoped group is granted over.
	pub fn accounts(&self) -> &BTreeSet<SmolStr> { &self.accounts }

	/// The zones every zone-scoped group is granted over, by domain.
	pub fn zones(&self) -> &BTreeMap<SmolStr, SmolStr> { &self.zones }

	/// What asks for the one group that can mint a credential wider than
	/// itself, [`None`] when nothing does. A lowering that answers [`None`] is
	/// a repo whose deploy token never needs to escalate, which is the whole
	/// point of asking.
	pub fn escalating(&self) -> Option<&BTreeSet<SmolStr>> {
		self.asked.get(&TokenPermission::API_TOKENS_WRITE)
	}

	/// The one account a minted token lives in, an error when the lowered
	/// stacks name none or several: an account-owned token is created under one
	/// account, so two would need two tokens and two documents to hold them.
	pub fn account(&self) -> Result<&SmolStr> {
		match self.accounts.iter().collect::<Vec<_>>().as_slice() {
			[account] => (*account).xok(),
			[] => bevybail!(
				"no cloudflare account is declared, so there is nowhere for a \
				token to live: declare `{{CloudflareAccount(\"..\")}}` on the \
				stack or an ancestor"
			),
			accounts => bevybail!(
				"the stacks name {} cloudflare accounts ({}), and one token \
				lives in one account: deploy them from separate repos, each \
				with its own document",
				accounts.len(),
				accounts
					.iter()
					.map(|account| account.as_str())
					.collect::<Vec<_>>()
					.join(", ")
			),
		}
	}

	/// The resource strings a group of `scope` is granted over, in the form a
	/// token policy names them.
	fn resources(&self, scope: TokenScope) -> Vec<String> {
		match scope {
			TokenScope::Account => self
				.accounts
				.iter()
				.map(|account| format!("com.cloudflare.api.account.{account}"))
				.collect(),
			TokenScope::Zone => self
				.zones
				.values()
				.map(|zone| format!("com.cloudflare.api.account.zone.{zone}"))
				.collect(),
			// never held by a token that deploys, see `TokenScope::Bucket`
			TokenScope::Bucket => Vec::new(),
		}
	}

	/// The `policies` of the token this lowers to, ready for
	/// `POST /accounts/{id}/tokens`: one allow policy per scope the lowering
	/// named, each over that scope's resources alone, so a zone group never
	/// reaches the account and an account group never reaches a zone.
	pub fn to_json(&self) -> Value {
		[TokenScope::Account, TokenScope::Zone]
			.into_iter()
			.filter_map(|scope| {
				let groups = self
					.asked
					.keys()
					.filter(|permission| permission.scope == scope)
					.map(|permission| json!({ "id": permission.id }))
					.collect::<Vec<_>>();
				if groups.is_empty() {
					return None;
				}
				let resources = self
					.resources(scope)
					.into_iter()
					.map(|resource| (resource, Value::from("*")))
					.collect::<serde_json::Map<_, _>>();
				json!({
					"effect": "allow",
					"resources": resources,
					"permission_groups": groups,
				})
				.xmap(Some)
			})
			.collect::<Vec<_>>()
			.xmap(Value::Array)
	}

	/// What the lowering grants, flattened to one `(resource, group id)` pair
	/// per combination: the form a held token is COMPARED in, since how
	/// Cloudflare groups the pairs into policies is not what either side means.
	pub fn fingerprint(&self) -> BTreeSet<(SmolStr, SmolStr)> {
		self.asked
			.keys()
			.flat_map(|permission| {
				self.resources(permission.scope)
					.into_iter()
					.map(|resource| (resource.into(), permission.id.into()))
			})
			.collect()
	}

	/// [`fingerprint`](Self::fingerprint) of the `policies` of a token
	/// Cloudflare answered with, so the two compare as sets. A policy that
	/// denies rather than allows is not a grant and is left out of both.
	pub fn fingerprint_of(policies: &Value) -> BTreeSet<(SmolStr, SmolStr)> {
		policies
			.as_array()
			.map(Vec::as_slice)
			.unwrap_or_default()
			.iter()
			.filter(|policy| policy["effect"] == "allow")
			.flat_map(|policy| {
				let groups = policy["permission_groups"]
					.as_array()
					.map(Vec::as_slice)
					.unwrap_or_default()
					.iter()
					.filter_map(|group| group["id"].as_str())
					.map(SmolStr::new)
					.collect::<Vec<_>>();
				policy["resources"]
					.as_object()
					.map(|resources| {
						resources.keys().cloned().collect::<Vec<_>>()
					})
					.unwrap_or_default()
					.into_iter()
					.flat_map(move |resource| {
						groups
							.clone()
							.into_iter()
							.map(move |group| (SmolStr::new(&resource), group))
					})
			})
			.collect()
	}
}

impl std::fmt::Display for DeployerToken {
	/// The dashboard recipe, in the order the Create Custom Token form asks for
	/// it: one line per group with what asked for it, then the resource list of
	/// each scope that HAS a group, which is what the minted policies carry. An
	/// account holding no account-scoped group is where the token LIVES rather
	/// than anything it reaches, so it is not listed as a resource.
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for (permission, askers) in &self.asked {
			let askers = askers
				.iter()
				.map(SmolStr::as_str)
				.collect::<Vec<_>>()
				.join(", ");
			writeln!(f, "permission: {permission} ({askers})")?;
		}
		for scope in [TokenScope::Account, TokenScope::Zone] {
			if !self.asked.keys().any(|held| held.scope == scope) {
				continue;
			}
			let described = match scope {
				TokenScope::Zone => self
					.zones
					.iter()
					.map(|(domain, id)| format!("{domain} ({id})"))
					.collect::<Vec<_>>(),
				_ => self
					.accounts
					.iter()
					.map(SmolStr::to_string)
					.collect::<Vec<_>>(),
			};
			writeln!(
				f,
				"{} resources: {}",
				scope.to_string().to_lowercase(),
				described.join(", ")
			)?;
		}
		Ok(())
	}
}

/// Which resource list a permission group is granted over. A token policy
/// names one resource string per scope, so the scope is also what decides
/// which address a lowering has to resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokenScope {
	/// `com.cloudflare.api.account.<id>`, the stack's [`CloudflareAccount`].
	Account,
	/// `com.cloudflare.api.account.zone.<id>`, the stack's [`CloudflareZone`].
	Zone,
	/// `com.cloudflare.edge.r2.bucket.<account>_default_<bucket>`, one bucket
	/// and nothing else in the account. Only ever MINTED into a token (the
	/// `R2BucketBlock` pair), never held by one that deploys.
	Bucket,
}

impl std::fmt::Display for TokenScope {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(match self {
			Self::Account => "Account",
			Self::Zone => "Zone",
			Self::Bucket => "Bucket",
		})
	}
}

/// One Cloudflare api-token permission group, by Cloudflare's own name for it
/// and the id a token policy names it by.
///
/// The ids are global constants rather than per-account, listed by
/// `GET /accounts/{id}/tokens/permission_groups`; one that stopped existing
/// fails the call that uses it, which is the same loud failure a renamed group
/// gives.
///
/// The names are the api's. The dashboard's Create Custom Token form spells
/// the same groups as `<noun>: Edit` where the api says `<noun> Write`, and
/// renames two outright: `Cache Settings` is `Cache Rules` there, and
/// `Workers Containers` is `Containers`. There is deliberately no second
/// spelling here, since a group has one id and that is the fact worth keeping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TokenPermission {
	/// What the group is granted over, which orders a printed list the way the
	/// dashboard asks for it: every account group, then every zone one.
	scope: TokenScope,
	/// Cloudflare's own name, ie `DNS Write`.
	name: &'static str,
	/// The account-wide constant id a token policy names the group by.
	id: &'static str,
}

impl TokenPermission {
	const fn account(name: &'static str, id: &'static str) -> Self {
		Self {
			scope: TokenScope::Account,
			name,
			id,
		}
	}

	const fn zone(name: &'static str, id: &'static str) -> Self {
		Self {
			scope: TokenScope::Zone,
			name,
			id,
		}
	}

	const fn bucket(name: &'static str, id: &'static str) -> Self {
		Self {
			scope: TokenScope::Bucket,
			name,
			id,
		}
	}

	/// Lists the accounts the token can see, which is how `wrangler` resolves
	/// an account it was given no id for.
	pub const ACCOUNT_SETTINGS_READ: Self = Self::account(
		"Account Settings Read",
		"c1fde68c7bcc44588cbb6ddbc16d6480",
	);

	/// Reads the account's api tokens, which a provider refresh of a minted
	/// token needs.
	pub const API_TOKENS_READ: Self = Self::account(
		"Account API Tokens Read",
		"eb56a6953c034b9d97dd838155666f06",
	);

	/// Creates, rewrites and deletes the account's api tokens: the one group
	/// that can mint a credential WIDER than the token holding it, since a
	/// minted token may carry any group the account has. Cloudflare has no
	/// boundary to condition it with, so nothing but keeping it out of a held
	/// credential limits it.
	pub const API_TOKENS_WRITE: Self = Self::account(
		"Account API Tokens Write",
		"5bc3f8b21c554832afc660159ab75fa4",
	);

	/// The compute the container applications run on, which `wrangler deploy`
	/// addresses alongside the registry image.
	pub const CLOUDCHAMBER_WRITE: Self =
		Self::account("Cloudchamber Write", "26ce6c7d18a346528e7b905d5e269866");

	/// The account half of a load balancer: its health monitors and its
	/// origin pools.
	pub const LOAD_BALANCER_POOLS_WRITE: Self = Self::account(
		"Load Balancing: Monitors and Pools Write",
		"d2a1802cc9a34e30852f8b33869b2f3c",
	);

	/// The container applications and their managed-registry images.
	pub const WORKERS_CONTAINERS_WRITE: Self = Self::account(
		"Workers Containers Write",
		"bdbcd690c763475a985e8641dddc09f7",
	);

	/// R2 buckets and their contents at the account level: creating one,
	/// deleting one, and `wrangler r2 object put`.
	pub const WORKERS_R2_STORAGE_WRITE: Self = Self::account(
		"Workers R2 Storage Write",
		"bf7481a1826f439697cb59a20b22293e",
	);

	/// Worker scripts, their bindings and their secrets: the `wrangler deploy`
	/// upload.
	pub const WORKERS_SCRIPTS_WRITE: Self = Self::account(
		"Workers Scripts Write",
		"e086da7e2179491d91ee5f35b3ca210a",
	);

	/// Purges the zone cache. Its own group, held by nothing else, and the one
	/// whose dashboard level reads `Purge` rather than `Edit`.
	pub const CACHE_PURGE: Self =
		Self::zone("Cache Purge", "e17beae8b8cb423a99b1730f21238bed");

	/// The cache-phase ruleset, ie the `http_request_cache_settings`
	/// entrypoint the zone setup publishes.
	pub const CACHE_SETTINGS_WRITE: Self =
		Self::zone("Cache Settings Write", "9ff81cbbe65c400b97d92c3c1033cab6");

	/// Every record in the zone: what an apply publishes and what an audit
	/// deletes.
	pub const DNS_WRITE: Self =
		Self::zone("DNS Write", "4755a26eedb94da69e1066d98aa820be");

	/// The zone half of a load balancer, steering one proxied hostname.
	pub const LOAD_BALANCERS_WRITE: Self =
		Self::zone("Load Balancers Write", "6d7f2f5f5b1d4a0e9081fdc98d432fd1");

	/// A Worker's routes and custom domains, which a `wrangler deploy`
	/// provisions with their record and certificate.
	pub const WORKERS_ROUTES_WRITE: Self =
		Self::zone("Workers Routes Write", "28f4b596e7d643029c524985477ae49a");

	/// The zone settings the setup patches, ie `ssl` and `always_use_https`.
	pub const ZONE_SETTINGS_WRITE: Self =
		Self::zone("Zone Settings Write", "3030687196b94b638145a3953da2b699");

	/// Reads the objects of one bucket, half of what a minted R2 token holds.
	pub const BUCKET_ITEM_READ: Self = Self::bucket(
		"Workers R2 Storage Bucket Item Read",
		"6a018a9f2fc74eb6b293b0c548f38b39",
	);

	/// Writes and deletes the objects of one bucket, the other half. R2 keeps
	/// no versions, so this is also the group that makes a delete final.
	pub const BUCKET_ITEM_WRITE: Self = Self::bucket(
		"Workers R2 Storage Bucket Item Write",
		"2efd5506f9c8494dacb1fa10a3e7d5b6",
	);

	/// What the group is granted over.
	pub const fn scope(&self) -> TokenScope { self.scope }

	/// Cloudflare's own name for the group.
	pub const fn name(&self) -> &'static str { self.name }

	/// The id a token policy names the group by.
	pub const fn id(&self) -> &'static str { self.id }
}

impl std::fmt::Display for TokenPermission {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} > {}", self.scope, self.name)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A stack with both Cloudflare addresses declared, which every lowering
	/// here resolves against.
	fn addressed() -> (ResolvedStack, Deployment, crate::types::TestWorkDir) {
		let (stack, deployment, dir) = ResolvedStack::default_local();
		(
			stack
				.with_cloudflare_account(CloudflareAccount::new("acct123"))
				.with_cloudflare_zone(CloudflareZone::new(
					"beetmash.com",
					"zone123",
				)),
			deployment,
			dir,
		)
	}

	/// A declared type's groups are its longest matching prefix's, a bucket's
	/// lifecycle rides the bucket's own entry, and a `cloudflare_` type with
	/// none is a loud error.
	#[beet_core::test]
	fn places_every_cloudflare_type() {
		let names = |declared| {
			DeployerToken::resource(declared)
				.unwrap()
				.iter()
				.map(TokenPermission::name)
				.collect::<Vec<_>>()
		};
		names("cloudflare_dns_record").xpect_eq(vec!["DNS Write"]);
		names("cloudflare_r2_bucket_lifecycle")
			.xpect_eq(vec!["Workers R2 Storage Write"]);
		names("cloudflare_r2_bucket_lock")
			.xpect_eq(vec!["Workers R2 Storage Write"]);
		// the longer prefix wins: a pool is account-scoped where the load
		// balancer naming it is zone-scoped
		names("cloudflare_load_balancer")
			.xpect_eq(vec!["Load Balancers Write"]);
		names("cloudflare_load_balancer_pool")
			.xpect_eq(vec!["Load Balancing: Monitors and Pools Write"]);
		DeployerToken::resource("cloudflare_queue")
			.unwrap_err()
			.to_string()
			.xpect_contains("cloudflare_queue");
	}

	/// What the frequent work needs: the three zone verbs every deploy runs ask
	/// for four zone groups, no account group at all, and nothing that can mint
	/// a credential. Read off the actions' own declarations, so a change to one
	/// of them is a change to this answer.
	#[cfg(all(
		feature = "mail",
		feature = "deploy",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	fn the_zone_verbs_never_escalate() {
		let (stack, ..) = addressed();
		let token = [
			CloudflareAccess::declared_by::<CloudflareZoneSetup>(),
			CloudflareAccess::declared_by::<CloudflarePurgeCache>(),
			CloudflareAccess::declared_by::<ZoneAudit>(),
		]
		.iter()
		.try_fold(DeployerToken::default(), |token, access| {
			token.lower_access(&stack, access)
		})
		.unwrap();
		token
			.asked()
			.keys()
			.map(TokenPermission::name)
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"Cache Purge",
				"Cache Settings Write",
				"DNS Write",
				"Zone Settings Write",
			]);
		// the account is where the token LIVES, never what it reaches: a zone
		// group's resource list names the zone alone
		token.account().unwrap().as_str().xpect_eq("acct123");
		token
			.to_json()
			.to_string()
			.as_str()
			.xpect_contains("com.cloudflare.api.account.zone.zone123")
			.xnot()
			.xpect_contains("\"com.cloudflare.api.account.acct123\"");
		token.escalating().is_none().xpect_true();
	}

	/// The body a mint posts: one allow policy per scope, each over that scope's
	/// own resources, and a fingerprint that compares equal however Cloudflare
	/// groups the same pairs back. A deny is not a grant and counts for
	/// neither side.
	#[beet_core::test]
	fn the_token_body_is_one_policy_per_scope() {
		struct Purge;
		struct Sync;
		let (stack, ..) = addressed();
		let token = [
			CloudflareAccess::new::<Purge>(&[TokenPermission::CACHE_PURGE]),
			CloudflareAccess::new::<Sync>(&[
				TokenPermission::ACCOUNT_SETTINGS_READ,
				TokenPermission::WORKERS_R2_STORAGE_WRITE,
			]),
		]
		.iter()
		.try_fold(DeployerToken::default(), |token, access| {
			token.lower_access(&stack, access)
		})
		.unwrap();
		let policies = token.to_json();
		let scoped = |permission: TokenPermission| {
			policies
				.as_array()
				.unwrap()
				.iter()
				.find(|policy| policy.to_string().contains(permission.id()))
				.unwrap()["resources"]
				.as_object()
				.unwrap()
				.keys()
				.cloned()
				.collect::<Vec<_>>()
		};
		// two scopes, two policies, neither reaching the other's resources
		policies.as_array().unwrap().len().xpect_eq(2);
		scoped(TokenPermission::CACHE_PURGE)
			.xpect_eq(vec!["com.cloudflare.api.account.zone.zone123"]);
		scoped(TokenPermission::WORKERS_R2_STORAGE_WRITE)
			.xpect_eq(vec!["com.cloudflare.api.account.acct123"]);
		DeployerToken::fingerprint_of(&policies).xpect_eq(token.fingerprint());
		// the same grants, one policy per pair, plus a deny: still a match
		let regrouped = serde_json::json!([
			{
				"effect": "allow",
				"resources": { "com.cloudflare.api.account.zone.zone123": "*" },
				"permission_groups": [{ "id": TokenPermission::CACHE_PURGE.id() }],
			},
			{
				"effect": "allow",
				"resources": { "com.cloudflare.api.account.acct123": "*" },
				"permission_groups": [
					{ "id": TokenPermission::ACCOUNT_SETTINGS_READ.id() },
					{ "id": TokenPermission::WORKERS_R2_STORAGE_WRITE.id() },
				],
			},
			{
				"effect": "deny",
				"resources": { "com.cloudflare.api.account.acct123": "*" },
				"permission_groups": [{ "id": TokenPermission::API_TOKENS_WRITE.id() }],
			},
		]);
		DeployerToken::fingerprint_of(&regrouped).xpect_eq(token.fingerprint());
	}

	/// A rendered record lowers to the one group that publishes it and names
	/// the zone it landed in; the AWS types beside it belong to another
	/// provider's policy and are skipped rather than refused.
	#[cfg(feature = "cloudflare_dns")]
	#[beet_core::test]
	fn lowers_a_rendered_record() {
		let (stack, deployment, _dir) = addressed();
		let mut config = deployment.create_config(&stack);
		DnsProvider::cloudflare("mail.beetmash.com")
			.emit_txt(
				&stack,
				&mut config,
				"spf",
				"mail.beetmash.com",
				"v=spf1 -all",
			)
			.unwrap();
		let token = DeployerToken::default().lower(&stack, &config).unwrap();
		token
			.to_string()
			.as_str()
			.xpect_contains(
				"permission: Zone > DNS Write (cloudflare_dns_record)",
			)
			.xpect_contains("zone resources: beetmash.com (zone123)");
		token.escalating().is_none().xpect_true();
	}

	/// The hole the credential split closed, from the lowering's side: a bucket
	/// renders its own storage group and NOTHING that mints, so no repo's deploy
	/// token escalates however many buckets it declares. `cloudflare/mint` mints
	/// the bucket's own token out of band.
	///
	/// The `cloudflare_account_token` entry stays in the table deliberately, and
	/// `places_every_cloudflare_type` covers it: if a declaration ever renders
	/// one again, the lowering names the escalating group out loud rather than
	/// letting it back in quietly.
	#[cfg(feature = "cloudflare_dns")]
	#[beet_core::test]
	fn an_r2_bucket_never_escalates() {
		let (stack, deployment, _dir) = addressed();
		let mut config = deployment.create_config(&stack);
		// its lock rides the same `cloudflare_r2_bucket` prefix entry, so the
		// retention a bucket must declare asks for no further group
		R2BucketBlock::new("cold-backups")
			.with_retain_days(30)
			.emit(&stack, &deployment, &mut config)
			.unwrap();
		let token = DeployerToken::default().lower(&stack, &config).unwrap();
		token.escalating().is_none().xpect_true();
		token
			.asked()
			.keys()
			.map(TokenPermission::name)
			.collect::<Vec<_>>()
			.xpect_eq(vec!["Workers R2 Storage Write"]);
		token
			.to_string()
			.as_str()
			.xpect_contains("account resources: acct123")
			// the stack declares a zone and the bucket needs nothing in it, so
			// the token reaches none
			.xnot()
			.xpect_contains("zone resources");
	}

	/// A zone-scoped group with no zone fails naming the spread; a missing
	/// ACCOUNT is not the lowering's complaint at all, since the list still
	/// answers and only the mint that needs somewhere to put a token says so.
	/// (An action that declares nothing is refused where it reaches for the
	/// token, see `CloudflareAccess`.)
	#[beet_core::test]
	fn a_missing_address_is_loud() {
		struct Purge;
		let purge =
			CloudflareAccess::new::<Purge>(&[TokenPermission::CACHE_PURGE]);
		let (bare, ..) = ResolvedStack::default_local();
		DeployerToken::default()
			.lower_access(&bare, &purge)
			.unwrap_err()
			.to_string()
			.xpect_contains("CloudflareZone");
		let zoned = ResolvedStack::default_local().0.with_cloudflare_zone(
			CloudflareZone::new("beetmash.com", "zone123"),
		);
		let lowered = DeployerToken::default()
			.lower_access(&zoned, &purge)
			.unwrap();
		lowered
			.asked()
			.keys()
			.map(TokenPermission::name)
			.collect::<Vec<_>>()
			.xpect_eq(vec!["Cache Purge"]);
		lowered
			.account()
			.unwrap_err()
			.to_string()
			.xpect_contains("CloudflareAccount")
			.xpect_contains("nowhere for a token to live");
	}
}

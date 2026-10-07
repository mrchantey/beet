//! `cloudflare/mint`: every Cloudflare credential this repo's declarations ask
//! for — the narrow token it deploys with, and the token of each bucket it
//! declares — minted in the one place that holds the group which mints.

use crate::actions::cloudflare_api_ext;
use crate::actions::cloudflare_api_ext::API_BASE;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;

/// Request params for [`CloudflareMint`], surfaced in `--help`.
#[derive(Reflect)]
struct CloudflareMintParams {
	/// Print the token this would create and what asked for each of its
	/// permissions, WRITING to neither Cloudflare nor the document. The stacks
	/// render, which needs no credential at all, so this is also how a reader
	/// sees the scope without holding anything.
	dry_run: bool,
	/// Mint a fresh token even when the account already holds one that matches
	/// the declarations, and delete the one it replaces: this is the rotation.
	rotate: bool,
	/// The document group the token is sealed in, `default` when absent.
	group: Option<String>,
}

/// `<CloudflareMint/>` — converge every Cloudflare credential this repo's
/// declarations ask for:
///
/// - **the deploy token**: one account-owned token per repo, scoped to exactly
///   the permission groups the declarations need ([`DeployerToken`], lowered
///   from what the stacks render and the Cloudflare actions their routes run),
///   sealed into the credential document as `CLOUDFLARE_API_TOKEN`.
/// - **one token per declared `<R2BucketBlock/>`**, scoped to that bucket's
///   objects and nothing else, its S3 pair parked in the stack's own secret
///   store where the compute beside the bucket reads it. Not sealed in any
///   document: it is a runtime credential, not a deploy one.
///
/// Both live here because both need the one group that mints credentials, and
/// that group is held by one credential in one place.
///
/// ```sh
/// beet cloudflare/mint --dry-run   # the token's scope, nothing touched
/// beet cloudflare/mint             # converge the token and seal it
/// beet cloudflare/mint --rotate    # ..and replace it even if it matches
/// ```
///
/// ## Which credential it runs as
///
/// Creating a token needs `Account API Tokens Write`, which is the one group
/// that can mint a credential wider than itself and therefore the one group a
/// held credential must not have. So this verb does not run as the token it
/// converges: the **mint token** is passed for one command and wins over the
/// document, exactly as an admin pair is passed to `deployer/mint`.
///
/// ```sh
/// CLOUDFLARE_API_TOKEN=<the mint token> beet cloudflare/mint
/// ```
///
/// The mint token holds `Account API Tokens Write` and nothing else, and it
/// lives in the password manager, in NO document. That absence is the whole
/// design: Cloudflare has no mfa condition to put on a token and no
/// permissions boundary to cap one with, so the only thing that keeps the
/// escalating group out of reach of a held key is that no sealed file carries
/// it. An agent with the age identity opens every document and still cannot
/// mint.
///
/// ## Why a verb rather than a hand-made token, and why not an apply
///
/// Against a hand-made token, the same reason `deployer/mint` is a verb: a
/// credential written by hand drifts from the declarations the moment a block
/// is added, and the drift surfaces as a 403 mid-deploy that names nothing
/// useful. Lowered from the render, the token is as narrow as the declarations
/// allow and widens only when they do.
///
/// Against an apply, which CAN mint a token (`cloudflare_account_token` is a
/// bound resource, and the R2 bucket's credential was rendered that way until
/// this verb existed): an apply that mints needs `Account API Tokens Write` in
/// the credential every deploy of that repo reads, which is exactly the
/// escalation with no boundary to cap it. Minting is rare and deploying is
/// constant, so they hold different credentials, and the rare one is the only
/// one that can mint. That is the whole shape of this file.
///
/// ## What converges
///
/// - the token, named `<repo>-deploy` after the workspace directory (a document
///   holds one token however many stacks it deploys), created unless the
///   account holds exactly one token of that name, active, granting what the
///   declarations ask for under the declared terms, and not yet inside its
///   renewal notice.
/// - the record, sealed as `CLOUDFLARE_API_TOKEN` in the document's group with
///   the token's `expires`. The value is written only after the new token has
///   PROVEN itself with a read the deploy actually makes, and every other token
///   of the name is deleted only after that: a failure anywhere leaves the
///   previous credential in place, and a token that never proved itself is
///   deleted on the spot rather than left live beside it.
///
/// - a bucket's token, minted unless the account holds exactly one token of its
///   name, its pair is parked, and the parked access key id is that token's, or
///   on `--rotate`. The parked access key id IS the token id, so that
///   comparison costs no call of its own.
///
/// What it deliberately does not do is delete a token it did not make. A
/// hand-made token is user-owned, which `Account API Tokens Write` cannot touch
/// at all, so the verb names it and the operator revokes it in the dashboard.
///
/// ## The two levers that are not a boundary
///
/// Cloudflare caps a token by nothing but its scope, so the deploy token is
/// also held to two terms that narrow the window a leaked value is good for:
///
/// - **a lifetime**, `ttl_days`: the token stops authenticating on a day, so a
///   leak has a deadline however it leaked. The sealed record carries that day
///   as its `expires`, every launch that loads it warns in the fortnight before
///   ([`SecretRecord::EXPIRY_NOTICE`]), and this verb renews inside the same
///   fortnight, so the deadline arrives as a warning naming one command rather
///   than as a 401 mid-deploy. Renewal is a fresh token, never an extended
///   one: an extension would keep a leaked value alive with it.
/// - **an address filter**, `request_ips`: the CIDRs the token may be used
///   from, any address when empty. Worth declaring only where every deploy
///   leaves from fixed addresses (a self-hosted CI runner, a static egress
///   box); a laptop's address changes with the network it is on, and each
///   change would cost a mint.
///
/// ```jsx
/// <CloudflareMint ttl_days=30 request_ips={["203.0.113.7/32"]}/>
/// ```
///
/// A bucket's token is held to neither: the box beside the bucket uses it
/// nightly and cannot renew it, and the bucket's lock is what bounds its
/// damage.
#[action]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
#[require(
	PathPartial = PathPartial::new("cloudflare/mint"),
	ParamsPartial = ParamsPartial::new::<CloudflareMintParams>()
)]
pub async fn CloudflareMint(
	/// The days a minted deploy token authenticates for, to the midnight UTC
	/// it lands on. At least twice the renewal notice, so a token spends most
	/// of its life quiet.
	#[field(default = 90u32)]
	ttl_days: u32,
	/// The CIDRs the deploy token may be used from (a bare address is its
	/// `/32` or `/128`), any address when empty.
	#[field]
	request_ips: Vec<SmolStr>,
	cx: ActionContext<Request>,
) -> Result<Response> {
	let params = cx.input.parse_params::<CloudflareMintParams>()?;
	let terms = TokenTerms::new(ttl_days, &request_ips, Timestamp::now())?;
	let lowered = CloudflareMint::lower(&cx.caller).await?;
	let name = CloudflareMint::token_name()?;
	if let Some(askers) = lowered.escalating() {
		warn!(
			"the lowered token carries `{}`, so this repo's deploy credential \
			can mint any token the account holds: asked for by {}. A bucket's \
			own token is the only thing that needs it, and an apply that mints \
			one is what puts it here",
			TokenPermission::API_TOKENS_WRITE,
			askers
				.iter()
				.map(SmolStr::as_str)
				.collect::<Vec<_>>()
				.join(", ")
		);
	}
	let mut report = match params.dry_run {
		true => vec![CloudflareMint::describe(&name, &lowered, &terms)?],
		false => vec![
			CloudflareMint::converge(
				&cx.caller, &name, &lowered, &terms, &params,
			)
			.await?,
		],
	};
	report.extend(CloudflareMint::buckets(&cx.caller, &params).await?);
	Response::ok_text(format!("{}\n", report.join("\n"))).xok()
}

impl CloudflareMint {
	/// The record the token is sealed under, which is the variable every
	/// Cloudflare consumer reads: the tofu provider, `wrangler` and the zone
	/// verbs all take it from the environment, so what the document holds is
	/// what they get.
	const RECORD: &'static str = "CLOUDFLARE_API_TOKEN";

	/// The `--group` default, the group a first mint seals into.
	const DEFAULT_GROUP: &'static str = SecretsDocument::DEFAULT_GROUP;

	/// Lower every stack this launch declares and every Cloudflare action its
	/// routes carry, in one world pass, so a stack that cannot render fails
	/// here rather than at the account.
	async fn lower(caller: &AsyncEntity) -> Result<DeployerToken> {
		caller
			.with_world(|world, _| {
				let rendered = RenderScope::render_all(world)?
					.into_iter()
					.map(RenderScope::finish)
					.collect::<Result<Vec<_>>>()?;
				// one launch renders one stage, and a stage may declare a block
				// another does not (the bucket whose token an apply mints), so
				// a token lowered from a non-prod launch can be short of what
				// the deployed stage needs. A Cloudflare 403 names no
				// permission group, so this warning is the only thing that
				// would ever explain one.
				let launch = BootstrapConfig::get().stage.clone();
				if launch != BootstrapConfig::PROD_STAGE
					&& let Some(stage) = rendered
						.iter()
						.map(|(stack, ..)| stack.stage())
						.find(|stage| **stage == launch)
				{
					warn!(
						"lowering the stage `{stage}` declarations: a stage \
						that declares more needs its own run, so mint under \
						the stage that deploys (`--stage={}`)",
						BootstrapConfig::PROD_STAGE
					);
				}
				let mut lowered = DeployerToken::default();
				for (stack, _deployment, config) in rendered.iter() {
					lowered = lowered.lower(stack, config)?;
				}
				for (entity, access) in Self::declared_access(world) {
					// the action's OWN stack: a route's verbs may sit outside
					// every `<Stack>`, resolving the addresses above them
					let stack = world.with_state::<StackQuery, _>(|stacks| {
						stacks.resolve(entity)
					});
					lowered = lowered.lower_access(&stack, &access)?;
				}
				if lowered.asked().is_empty() {
					bevybail!(
						"this launch declares nothing at Cloudflare: no stack \
						renders a `cloudflare_` resource and no route runs a \
						Cloudflare action, so there is no token to mint"
					);
				}
				lowered.xok()
			})
			.await?
	}

	/// Every Cloudflare action this launch runs, by the [`CloudflareAccess`]
	/// each one declares. No list of action names stands between the two: the
	/// declaration is how an action reaches the token at all, so an action that
	/// calls Cloudflare is one this finds.
	fn declared_access(world: &mut World) -> Vec<(Entity, CloudflareAccess)> {
		world
			.query::<(Entity, &CloudflareAccess)>()
			.iter(world)
			.map(|(entity, access)| (entity, *access))
			.collect()
	}

	/// The token every launch of this repo deploys with: the workspace
	/// directory, kebab-cased, and `-deploy`. Named for the repo rather than for
	/// an app, since a credential document holds one token however many apps it
	/// deploys, and derived from the directory because a repo has no other name
	/// ([`DeployerMint::user_name`] names the AWS deployer the same way).
	fn token_name() -> Result<String> {
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
					"cannot name a token after the workspace directory `{}`: \
					a token name is alphanumeric",
					root.display()
				)
			})?;
		format!("{name}-deploy").xok()
	}

	/// The dry run's answer: the token, its terms, every group with what asked
	/// for it, and the body a mint would post, pretty printed so it reads and
	/// pipes.
	///
	/// An entry declaring no account still gets the whole list, and the one line
	/// it is missing instead of a token: what an entry ASKS FOR is worth reading
	/// whether or not anybody has said where its token would live.
	fn describe(
		name: &str,
		lowered: &DeployerToken,
		terms: &TokenTerms,
	) -> Result<String> {
		let home = match lowered.account() {
			Ok(account) => format!("account {account}"),
			Err(err) => format!("NOT MINTABLE: {err}"),
		};
		format!(
			"token {name}\n{home}\n{terms}\n\n{lowered}\nbody\n{}\n",
			serde_json::to_string_pretty(&terms.body(name, lowered.to_json()))?
		)
		.xok()
	}

	/// Converge the token: mint one unless the account holds exactly one of
	/// this name that grants what the declarations ask for under the declared
	/// terms, is active and not yet due, and is the one the document holds; or
	/// on `--rotate`. The new value is sealed only once proven, and every other
	/// token of the name deleted only once sealed.
	async fn converge(
		caller: &AsyncEntity,
		name: &str,
		lowered: &DeployerToken,
		terms: &TokenTerms,
		params: &CloudflareMintParams,
	) -> Result<String> {
		let account = lowered.account()?.clone();
		let handle = SecretsHandle::resolve(caller, None).await?;
		let identity = AgeIdentityFile::require()?;
		let mut document = handle.read_or_new().await?;
		let held = document.open(&identity).ok().and_then(|opened| {
			opened.get(Self::RECORD).map(|secret| secret.value.clone())
		});
		let existing = Self::find_tokens(&account, name).await?;
		// a second token of the name is never current: it is a mint a failure
		// interrupted, and minting again is what deletes it
		let current = match existing.as_slice() {
			[held_token] => {
				held_token.active
					&& DeployerToken::fingerprint_of(&held_token.policies)
						== lowered.fingerprint()
					&& terms.kept_by(held_token, Timestamp::now())
					&& Self::holds(&account, held.as_deref(), &held_token.id)
						.await
			}
			_ => false,
		};
		if current && !params.rotate {
			return format!(
				"token {name} ({}) matches the declarations, expires {}, and is \
				sealed in {}; `--rotate` mints another",
				existing[0].id,
				existing[0]
					.expires_on
					.map(|at| Date::from(at).to_string())
					.unwrap_or_default(),
				handle.describe()
			)
			.xok();
		}
		let (id, value) =
			Self::create_token(&account, terms.body(name, lowered.to_json()))
				.await?;
		let (url, what) = Self::deploy_proof(lowered)?;
		Self::prove_or_discard(&account, &id, &value, url, what).await?;
		let group = params.group.as_deref().unwrap_or(Self::DEFAULT_GROUP);
		document.set(&identity, group, Self::RECORD, &value, SecretRecord {
			role: Some(SecretRole::EnvVar),
			note: Some(
				format!(
					"cloudflare deploy token `{name}` ({id}): account-owned, \
					scoped to exactly what this repo's stacks and routes \
					declare; every tofu apply, wrangler call and zone verb \
					reads it from here"
				)
				.into(),
			),
			expires: Some(terms.expires_on),
			rotation: Some(Self::rotation()),
			..default()
		})?;
		handle.write(&document).await?;
		// only now: the token that replaces them is sealed and proven
		let replaced = Self::delete_others(&account, &existing, &id).await?;
		format!(
			"token {name} ({id}) minted, expiring {}, and sealed in {} (group \
			`{group}`){replaced}",
			Date::from(terms.expires_on),
			handle.describe(),
		)
		.xok()
	}

	/// How the token rotates: this verb again, which only the mint token can
	/// run. The first line is the command an expiry notice quotes.
	fn rotation() -> SecretRotation {
		SecretRotation::manual(
			"beet cloudflare/mint\n> with the mint token in the environment, \
			which wins over this document: `CLOUDFLARE_API_TOKEN=.. beet \
			cloudflare/mint`\n> renews the token inside its last fortnight and \
			replaces it whenever the declarations change; `--rotate` replaces \
			it at any time\n> the mint token holds `Account API Tokens Write` \
			and nothing else, and lives in the password manager rather than in \
			any document: it is the one credential the age key must not \
			open\n> the new token is minted, proven with a read this repo's \
			deploy makes and sealed, then the token it replaces is deleted",
		)
	}

	/// Every token of the account named `name`, empty when it holds none.
	/// Paged to the end: a token past the first page would read as absent,
	/// and the verb would mint a second token of the same name on every run.
	async fn find_tokens(account: &str, name: &str) -> Result<Vec<HeldToken>> {
		const PER_PAGE: usize = 50;
		let mut found = Vec::new();
		for page in 1.. {
			let body = Self::send(
				Request::get(format!(
					"{API_BASE}/accounts/{account}/tokens?per_page={PER_PAGE}&page={page}"
				)),
				"listing the account's api tokens",
			)
			.await?;
			found.extend(
				body["result"]
					.as_array()
					.map(Vec::as_slice)
					.unwrap_or_default()
					.iter()
					.filter(|token| token["name"] == name)
					.filter_map(HeldToken::parse),
			);
			let pages =
				body["result_info"]["total_pages"].as_u64().unwrap_or(1);
			if page >= pages.max(1) {
				break;
			}
		}
		found.xok()
	}

	/// Delete every token of `existing` but the one just sealed, answering
	/// the report's `, replacing ..` clause.
	async fn delete_others(
		account: &str,
		existing: &[HeldToken],
		kept: &str,
	) -> Result<String> {
		let mut replaced = Vec::new();
		for held_token in existing.iter().filter(|token| token.id != kept) {
			Self::delete_token(account, &held_token.id).await?;
			replaced.push(held_token.id.as_str());
		}
		match replaced.is_empty() {
			true => String::new(),
			false => format!(", replacing {}", replaced.join(", ")),
		}
		.xok()
	}

	/// Whether the document's value IS the token the account holds, which is
	/// what makes a converge a no-op rather than only a matching scope
	/// somewhere.
	///
	/// A verify the endpoint does not answer leaves this `true`: the scope
	/// already matched, and a token that cannot be identified is not a reason
	/// to mint over a working deploy. An answer naming another token, or a
	/// refusal, is.
	async fn holds(account: &str, held: Option<&str>, id: &str) -> bool {
		let Some(held) = held else { return false };
		let verified = cloudflare_api_ext::send_optional(
			Request::get(format!(
				"{API_BASE}/accounts/{account}/tokens/verify"
			))
			.with_auth_bearer(held),
			"verifying the sealed token",
		)
		.await;
		match verified {
			Ok(Some(body)) => body["result"]["id"] == id,
			Ok(None) => true,
			Err(_) => false,
		}
	}

	/// Create the token `body` describes (its `name`, `policies` and any
	/// terms), answering its id and its value. The value is the only copy
	/// Cloudflare will ever hand over, so nothing here logs the answer and a
	/// failure is reported through its `errors` alone.
	async fn create_token(
		account: &str,
		body: Value,
	) -> Result<(SmolStr, SmolStr)> {
		let name = body["name"].as_str().unwrap_or_default().to_string();
		let response = Self::authed(Request::post(format!(
			"{API_BASE}/accounts/{account}/tokens"
		)))?
		.with_json_body(&body)?
		.send()
		.await?;
		let status = response.status();
		let body = serde_json::from_str::<Value>(
			&response.text().await.unwrap_or_default(),
		)
		.unwrap_or_default();
		if !status.is_ok() || body["success"] != true {
			bevybail!(
				"minting the token failed: {status} - {}{}",
				cloudflare_api_ext::error_messages(&body),
				match status.as_u16() {
					401 | 403 =>
						"\nthis credential may not edit the account's \
						api tokens, and a deploy token never may: pass the \
						mint token for this one command \
						(`CLOUDFLARE_API_TOKEN=.. beet cloudflare/mint`), \
						which wins over the document",
					_ => "",
				}
			);
		}
		match (
			body["result"]["id"].as_str(),
			body["result"]["value"].as_str(),
		) {
			(Some(id), Some(value)) => {
				(SmolStr::new(id), SmolStr::new(value)).xok()
			}
			// a token whose value was not answered is unusable and unsealed, so
			// it is named rather than left behind silently
			_ => bevybail!(
				"the minted token answered no value, so nothing was sealed: \
				delete `{name}` in the dashboard and run this again"
			),
		}
	}

	/// Prove a freshly minted token before anything relies on it, with a read
	/// `what` describes: one page of the zone's records for a token that
	/// deploys, its own verify for one scoped to a bucket's objects (whose S3
	/// pair is a derivation no bearer call can exercise).
	///
	/// Retried rather than slept through, since a new token answers `1000
	/// Invalid API Token` for a few seconds. A failure here leaves the previous
	/// credential in place, sealed or parked.
	async fn prove_token(value: &str, url: String, what: &str) -> Result {
		const ATTEMPTS: usize = 10;
		let mut last = None;
		for attempt in 0..ATTEMPTS {
			match cloudflare_api_ext::send(
				Request::get(url.clone()).with_auth_bearer(value),
				"proving the minted token",
			)
			.await
			{
				Ok(_) => return OK,
				Err(err) => {
					last = Some(err);
					time_ext::sleep_secs(1 + attempt as u64 / 3).await;
				}
			}
		}
		bevybail!(
			"the minted token never managed to {what}, so nothing was sealed or \
			parked and the previous one is untouched: {}",
			last.map(|err| err.to_string()).unwrap_or_default()
		)
	}

	/// [`prove_token`](Self::prove_token), deleting the new token `id` when it
	/// fails: unproven and unsealed, it would stay live beside the token it was
	/// to replace, under the same name, with a value nobody holds.
	async fn prove_or_discard(
		account: &str,
		id: &str,
		value: &str,
		url: String,
		what: &str,
	) -> Result {
		let Err(err) = Self::prove_token(value, url, what).await else {
			return OK;
		};
		match Self::delete_token(account, id).await {
			Ok(()) => Err(err),
			Err(delete_err) => bevybail!(
				"{err}\nand deleting the unproven token {id} failed too, so \
				delete it in the dashboard: {delete_err}"
			),
		}
	}

	/// Where a deploy token proves itself: one page of the zone's records, the
	/// read every plan of it makes, or its own verify when it is scoped to no
	/// zone at all.
	fn deploy_proof(lowered: &DeployerToken) -> Result<(String, &'static str)> {
		match lowered.zones().values().next() {
			Some(zone) => (
				format!("{API_BASE}/zones/{zone}/dns_records?per_page=1"),
				"read the zone's records",
			),
			None => (
				format!(
					"{API_BASE}/accounts/{}/tokens/verify",
					lowered.account()?
				),
				"verify itself",
			),
		}
		.xok()
	}

	async fn delete_token(account: &str, id: &str) -> Result {
		Self::send(
			Request::delete(format!(
				"{API_BASE}/accounts/{account}/tokens/{id}"
			)),
			"deleting the replaced api token",
		)
		.await
		.map(|_| ())
	}

	/// `request` carrying the credential this verb runs as, which is the mint
	/// token rather than the one it converges.
	fn authed(request: Request) -> Result<Request> {
		cloudflare_api_ext::token()
			.map(|token| request.with_auth_bearer(&token))
	}

	/// An api call as the mint token, whose `Account API Tokens Write` failure
	/// is re-raised as what to do about it.
	async fn send(request: Request, what: &str) -> Result<Value> {
		cloudflare_api_ext::send(Self::authed(request)?, what)
			.await
			.map_err(|err| match err.to_string().contains("403") {
				true => bevyhow!(
					"this credential may not read or edit the account's api \
					tokens, and a deploy token never may: pass the mint token \
					for this one command (`CLOUDFLARE_API_TOKEN=.. beet \
					cloudflare/mint`), which wins over the document. {err}"
				),
				false => err,
			})
	}

	/// Converge the token of every `<R2BucketBlock/>` this launch declares, or
	/// describe what that would do under `--dry-run`. One report line per
	/// bucket, none at all for a launch that declares none.
	///
	/// A bucket's token is not a deploy credential and is never sealed in a
	/// document: its S3 pair is parked in the stack's own secret store, where
	/// the compute beside the bucket reads it under its own grants.
	#[cfg(feature = "cloudflare_dns")]
	async fn buckets(
		caller: &AsyncEntity,
		params: &CloudflareMintParams,
	) -> Result<Vec<String>> {
		let mut report = Vec::new();
		let declared = caller
			.with_world(|world, _| Self::declared_buckets(world))
			.await??;
		for (block, stack, store) in declared {
			report.push(match params.dry_run {
				true => format!(
					"bucket token {} over {}, parked at {} and {}",
					block.token_name(&stack),
					block.bucket_name(&stack),
					store.address(&block.access_key_secret()),
					store.address(&block.secret_key_secret()),
				),
				false => {
					Self::converge_bucket(&block, &stack, &store, params)
						.await?
				}
			});
		}
		report.xok()
	}

	/// Every declared bucket with the stack it belongs to and the secret store
	/// that stack keeps its secrets in. A concrete query rather than the
	/// registry walk the actions need, since a block is one type.
	#[cfg(feature = "cloudflare_dns")]
	fn declared_buckets(
		world: &mut World,
	) -> Result<Vec<(R2BucketBlock, ResolvedStack, SecretStore)>> {
		let declared =
			world.with_state::<Query<(Entity, &R2BucketBlock)>, _>(|blocks| {
				blocks
					.iter()
					.map(|(entity, block)| (entity, block.clone()))
					.collect::<Vec<_>>()
			});
		let mut resolved = Vec::new();
		for (entity, block) in declared {
			let (stack, store) =
				world.with_state::<StackQuery, _>(|stacks| -> Result<_> {
					(stacks.resolve(entity), stacks.secret_store(entity)?).xok()
				})?;
			resolved.push((block, stack, store));
		}
		resolved.xok()
	}

	/// Converge one bucket's token: mint and park unless the account holds
	/// exactly one token of its name, the pair is parked and the parked access
	/// key id is that token's; or on `--rotate`.
	///
	/// The parked access key id IS the token id, so the comparison needs no
	/// call beyond the listing every converge makes anyway.
	#[cfg(feature = "cloudflare_dns")]
	async fn converge_bucket(
		block: &R2BucketBlock,
		stack: &ResolvedStack,
		store: &SecretStore,
		params: &CloudflareMintParams,
	) -> Result<String> {
		let account = stack.cloudflare_account()?.id().to_string();
		let name = block.token_name(stack);
		let (access_ref, secret_ref) =
			(block.access_key_secret(), block.secret_key_secret());
		let parked =
			(store.get(&access_ref).await?, store.get(&secret_ref).await?);
		let existing = Self::find_tokens(&account, &name).await?;
		let current = match (&parked, existing.as_slice()) {
			((Some(access_key), Some(_)), [held_token]) => {
				held_token.active && held_token.id == *access_key
			}
			_ => false,
		};
		if current && !params.rotate {
			return format!(
				"bucket token {name} is parked at {} and {}, `--rotate` mints \
				another",
				store.address(&access_ref),
				store.address(&secret_ref)
			)
			.xok();
		}
		let (id, value) = Self::create_token(
			&account,
			serde_json::json!({
				"name": name,
				"policies": block.token_policies(stack)?,
			}),
		)
		.await?;
		Self::prove_or_discard(
			&account,
			&id,
			&value,
			format!("{API_BASE}/accounts/{account}/tokens/verify"),
			"verify itself",
		)
		.await?;
		// the SECRET half first, and the order is the recovery: a failure
		// between the two writes leaves an id that is not the new token's
		// beside it, which is never current, so the next run mints again. The
		// other order would leave the new token's id beside the old secret, a
		// pair that looks current and is not.
		store
			.overwrite(
				&secret_ref,
				&cloudflare_api_ext::derive_secret_key(&value),
				Some(&block.secret_key_note()),
				Some(Self::bucket_rotation()),
			)
			.await?;
		store
			.overwrite(
				&access_ref,
				&id,
				Some(&block.access_key_note()),
				Some(Self::bucket_rotation()),
			)
			.await
			.map_err(|err| {
				bevyhow!(
					"parked the secret half of {name} but not its access key \
					id, so the parked pair authenticates nowhere until \
					`cloudflare/mint` runs again: {err}"
				)
			})?;
		// only now: the pair that replaces them is parked
		let replaced = Self::delete_others(&account, &existing, &id).await?;
		format!(
			"bucket token {name} ({id}) minted and parked at {} and {}{replaced}",
			store.address(&access_ref),
			store.address(&secret_ref),
		)
		.xok()
	}

	/// A launch built without the Cloudflare bindings declares no bucket, so
	/// there is nothing to converge.
	#[cfg(not(feature = "cloudflare_dns"))]
	async fn buckets(
		_caller: &AsyncEntity,
		_params: &CloudflareMintParams,
	) -> Result<Vec<String>> {
		Vec::new().xok()
	}

	/// How a bucket's parked pair rotates: this verb again, which replaces the
	/// token and re-parks both halves. The S3 secret is the SHA-256 of the
	/// token value, so neither half survives the other.
	#[cfg(feature = "cloudflare_dns")]
	fn bucket_rotation() -> SecretRotation {
		SecretRotation::manual(
			"beet cloudflare/mint --rotate\n> with the mint token in the \
			environment: the bucket's token is replaced, both halves re-parked \
			and the old token deleted",
		)
	}
}

/// What the deploy token is held to besides its scope: the instant it stops
/// authenticating and the addresses it may be used from.
#[derive(Debug, Clone, PartialEq)]
struct TokenTerms {
	/// Midnight UTC `ttl_days` after the mint.
	expires_on: Timestamp,
	/// Normalized CIDRs, sorted, empty for any address.
	request_ips: Vec<SmolStr>,
}

impl TokenTerms {
	const SECS_PER_DAY: u64 = 24 * 60 * 60;

	/// The terms a token minted at `now` is held to, refusing a lifetime the
	/// renewal notice would swallow and an address that is not one.
	fn new(
		ttl_days: u32,
		request_ips: &[SmolStr],
		now: Timestamp,
	) -> Result<Self> {
		let notice_days =
			SecretRecord::EXPIRY_NOTICE.as_secs() / Self::SECS_PER_DAY;
		if u64::from(ttl_days) < notice_days * 2 {
			bevybail!(
				"`ttl_days={ttl_days}` is under twice the {notice_days} day \
				notice a token is renewed inside, so it would spend most of its \
				life due for renewal: declare at least {}",
				notice_days * 2
			);
		}
		let mut request_ips = request_ips
			.iter()
			.map(|cidr| Self::cidr(cidr))
			.collect::<Result<Vec<_>>>()?;
		request_ips.sort();
		request_ips.dedup();
		let lifetime =
			Duration::from_secs(u64::from(ttl_days) * Self::SECS_PER_DAY);
		Self {
			expires_on: Date::from(now + lifetime).timestamp(),
			request_ips,
		}
		.xok()
	}

	/// `text` as a CIDR in one spelling, a bare address as its host route:
	/// the form a declaration and a listed token are both compared in.
	fn cidr(text: &str) -> Result<SmolStr> {
		let text = text.trim();
		let (address, prefix) = match text.split_once('/') {
			Some((address, prefix)) => (address, Some(prefix)),
			None => (text, None),
		};
		let address = address.parse::<core::net::IpAddr>().map_err(|_| {
			bevyhow!(
				"`{text}` is not an address or a CIDR, ie `203.0.113.7/32`"
			)
		})?;
		let widest = if address.is_ipv4() { 32 } else { 128 };
		let prefix = match prefix {
			None => widest,
			Some(prefix) => prefix
				.parse::<u8>()
				.ok()
				.filter(|prefix| *prefix <= widest)
				.ok_or_else(|| {
					bevyhow!("`{text}`: a prefix is 0 to {widest}")
				})?,
		};
		SmolStr::from(format!("{address}/{prefix}")).xok()
	}

	/// Whether `held` was minted under these terms and is not yet due: it
	/// expires after the renewal notice and no later than a token minted now
	/// would, and is usable from exactly the declared addresses. A token that
	/// never expires outlives every declaration, so it is never kept.
	fn kept_by(&self, held: &HeldToken, now: Timestamp) -> bool {
		held.expires_on.is_some_and(|at| {
			at > now + SecretRecord::EXPIRY_NOTICE && at <= self.expires_on
		}) && held.request_ips == self.request_ips
	}

	/// The create body of a token named `name` granting `policies` under
	/// these terms.
	fn body(&self, name: &str, policies: Value) -> Value {
		let mut body = serde_json::json!({
			"name": name,
			"policies": policies,
			// midnight by construction, so the day spells it exactly
			"expires_on": format!("{}T00:00:00Z", Date::from(self.expires_on)),
		});
		if !self.request_ips.is_empty() {
			body["condition"] =
				serde_json::json!({ "request_ip": { "in": self.request_ips } });
		}
		body
	}
}

/// The dry run's lines: when a token minted now would expire, and where from
/// it could be used.
impl core::fmt::Display for TokenTerms {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		write!(
			formatter,
			"expires {}, renewed by a mint inside its last {} days\nusable from {}",
			Date::from(self.expires_on),
			SecretRecord::EXPIRY_NOTICE.as_secs() / Self::SECS_PER_DAY,
			match self.request_ips.is_empty() {
				true => "any address".to_string(),
				false => self.request_ips.join(", "),
			}
		)
	}
}

/// The account's own token of a name, as a listing answers it: what a converge
/// compares the declarations against.
struct HeldToken {
	id: SmolStr,
	/// Its `policies`, compared through [`DeployerToken::fingerprint_of`] since
	/// how Cloudflare groups the pairs is not what either side means.
	policies: Value,
	/// A token may be disabled at the account, which no scope comparison would
	/// notice and every deploy would.
	active: bool,
	/// When it stops authenticating, [`None`] for a token that never does.
	expires_on: Option<Timestamp>,
	/// Its `condition.request_ip.in`, normalized as [`TokenTerms`] holds them.
	request_ips: Vec<SmolStr>,
}

impl HeldToken {
	fn parse(listed: &Value) -> Option<Self> {
		let mut request_ips = listed["condition"]["request_ip"]["in"]
			.as_array()
			.map(Vec::as_slice)
			.unwrap_or_default()
			.iter()
			.filter_map(Value::as_str)
			.filter_map(|cidr| TokenTerms::cidr(cidr).ok())
			.collect::<Vec<_>>();
		request_ips.sort();
		request_ips.dedup();
		Self {
			id: listed["id"].as_str()?.into(),
			policies: listed["policies"].clone(),
			active: listed["status"] == "active",
			expires_on: listed["expires_on"]
				.as_str()
				.and_then(Timestamp::parse_rfc3339),
			request_ips,
		}
		.xmap(Some)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Every Cloudflare action in the scene is found by the declaration it
	/// carries, so an action is lowered by being written down rather than by
	/// being listed somewhere else.
	///
	/// REGRESSION: the MTA-STS publish reached Cloudflare through wrangler but
	/// sat in no table of action names, so a token minted for a mail repo would
	/// have lacked its Workers groups and the next mail deploy would have
	/// answered 403 while publishing the policy.
	#[cfg(feature = "mail")]
	#[beet_core::test]
	fn finds_every_declared_action() {
		let mut world = InfraPlugin.into_world();
		world.spawn(CloudflarePurgeCache::default());
		world.spawn(MtaStsPublish::default());
		world.spawn(AwsRegion::new("us-west-2"));
		let mut found = super::CloudflareMint::declared_access(&mut world)
			.into_iter()
			.map(|(_, access)| access)
			.collect::<Vec<_>>();
		found.sort_by_key(|access| access.action());
		found
			.iter()
			.map(CloudflareAccess::action)
			.collect::<Vec<_>>()
			.xpect_eq(vec!["CloudflarePurgeCache", "MtaStsPublish"]);
		found[1]
			.permissions()
			.contains(&TokenPermission::WORKERS_SCRIPTS_WRITE)
			.xpect_true();
	}

	/// Every declared bucket is found with the stack above it and the store that
	/// stack parks its secrets in, so a bucket anywhere in the scene gets its
	/// token converged. The addresses are the ones the box reads under its own
	/// grants, which is why they are asserted here rather than taken on trust.
	#[cfg(feature = "cloudflare_dns")]
	#[beet_core::test]
	fn finds_every_declared_bucket() {
		use crate::types::test_support::*;
		let mut world = infra_world();
		world.spawn((
			Stack::new("mail").with_stage("prod"),
			// the default store is the stack's own ssm prefix, which is
			// regional, so the addresses below need the region declared
			AwsRegion::new("us-west-2"),
			CloudflareAccount::new("acct123"),
			children![R2BucketBlock::new("cold-backups")],
		));
		world.flush();
		let declared = CloudflareMint::declared_buckets(&mut world).unwrap();
		let (block, stack, store) = declared.into_iter().next().unwrap();
		block
			.token_name(&stack)
			.xpect_eq("mail--prod--cold-backups-token");
		store
			.address(&block.access_key_secret())
			.as_str()
			.xpect_contains("/mail/prod/cold-backups-access-key-id");
		store
			.address(&block.secret_key_secret())
			.as_str()
			.xpect_contains("/mail/prod/cold-backups-secret-access-key");
	}

	/// The token name is the repo's, not an app's: one document holds one token.
	#[beet_core::test]
	fn names_the_token_after_the_repo() {
		CloudflareMint::token_name()
			.unwrap()
			.xpect_eq("beet-deploy");
	}
	/// A mid-afternoon mint, so a lifetime visibly lands on a midnight.
	fn minted_at() -> Timestamp {
		Date::parse("2026-10-07").unwrap().timestamp()
			+ Duration::from_secs(15 * 60 * 60)
	}

	fn days(days: u64) -> Duration { Duration::from_secs(days * 24 * 60 * 60) }

	/// A held token as a listing answers it, expiring at `expires_on`.
	fn held(
		expires_on: Option<&str>,
		request_ips: &[&str],
	) -> super::HeldToken {
		super::HeldToken::parse(&serde_json::json!({
			"id": "abc",
			"status": "active",
			"policies": [],
			"expires_on": expires_on,
			"condition": { "request_ip": { "in": request_ips } },
		}))
		.unwrap()
	}

	/// The create body carries the lifetime as the midnight it lands on, and
	/// an address filter only when one is declared, normalized.
	#[beet_core::test]
	fn the_body_carries_the_terms() {
		let terms = super::TokenTerms::new(90, &[], minted_at()).unwrap();
		let body = terms.body("beet-deploy", serde_json::json!([]));
		body["expires_on"].xpect_eq("2027-01-05T00:00:00Z");
		body.get("condition").xpect_none();
		let terms = super::TokenTerms::new(
			90,
			&["203.0.113.7".into(), "2606:4700:0::/32".into()],
			minted_at(),
		)
		.unwrap();
		terms.body("beet-deploy", serde_json::json!([]))["condition"].xpect_eq(
			serde_json::json!({ "request_ip": { "in": [
				"203.0.113.7/32",
				"2606:4700::/32",
			]}}),
		);
	}

	/// A lifetime the renewal notice would mostly swallow is refused naming
	/// the floor, as is anything that is not an address.
	#[beet_core::test]
	fn refuses_terms_that_cannot_hold() {
		super::TokenTerms::new(27, &[], minted_at())
			.unwrap_err()
			.to_string()
			.xpect_contains("at least 28");
		super::TokenTerms::new(28, &[], minted_at()).unwrap();
		for cidr in ["beet.org", "203.0.113.7/33", "::1/129", "10.0.0.1/"] {
			super::TokenTerms::new(90, &[cidr.into()], minted_at())
				.unwrap_err();
		}
	}

	/// A held token is kept only while it is minted under the declared terms
	/// and not yet due: one that never expires, one inside its notice, one
	/// outliving a shortened lifetime and one usable from elsewhere are all
	/// minted again.
	#[beet_core::test]
	fn keeps_only_a_token_under_the_terms_and_not_yet_due() {
		let terms = super::TokenTerms::new(90, &[], minted_at()).unwrap();
		// the token a mint this afternoon would make, read back
		terms
			.kept_by(&held(Some("2027-01-05T00:00:00Z"), &[]), minted_at())
			.xpect_true();
		// the same token sixty days on, still outside its notice
		let later = minted_at() + days(60);
		super::TokenTerms::new(90, &[], later)
			.unwrap()
			.kept_by(&held(Some("2027-01-05T00:00:00Z"), &[]), later)
			.xpect_true();
		// and eighty days on, inside it
		let due = minted_at() + days(80);
		super::TokenTerms::new(90, &[], due)
			.unwrap()
			.kept_by(&held(Some("2027-01-05T00:00:00Z"), &[]), due)
			.xpect_false();
		// a token that never expires, the shape of every one minted before
		// the lifetime existed
		terms.kept_by(&held(None, &[]), minted_at()).xpect_false();
		// a lifetime shortened under a token that outlives it
		super::TokenTerms::new(30, &[], minted_at())
			.unwrap()
			.kept_by(&held(Some("2027-01-05T00:00:00Z"), &[]), minted_at())
			.xpect_false();
		// usable from an address the declaration no longer names
		terms
			.kept_by(
				&held(Some("2027-01-05T00:00:00Z"), &["203.0.113.7/32"]),
				minted_at(),
			)
			.xpect_false();
		super::TokenTerms::new(90, &["203.0.113.7".into()], minted_at())
			.unwrap()
			.kept_by(
				&held(Some("2027-01-05T00:00:00Z"), &["203.0.113.7/32"]),
				minted_at(),
			)
			.xpect_true();
	}
}

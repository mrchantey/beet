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
///   holds one token however many stacks it deploys), created when the account
///   holds none by that name and when the one it holds grants something other
///   than what the declarations ask for.
/// - the record, sealed as `CLOUDFLARE_API_TOKEN` in the document's group. The
///   value is written only after the new token has PROVEN itself with a read
///   the deploy actually makes, and the token it replaces is deleted only after
///   that: a failure anywhere leaves the previous credential in place.
///
/// - a bucket's token, minted when its pair is not parked, when the parked
///   access key id is not the account's live token of that name, or on
///   `--rotate`. The parked access key id IS the token id, so that comparison
///   costs no call of its own.
///
/// What it deliberately does not do is delete a token it did not make. A
/// hand-made token is user-owned, which `Account API Tokens Write` cannot touch
/// at all, so the verb names it and the operator revokes it in the dashboard.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(
	PathPartial = PathPartial::new("cloudflare/mint"),
	ParamsPartial = ParamsPartial::new::<CloudflareMintParams>()
)]
pub async fn CloudflareMint(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<CloudflareMintParams>()?;
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
		true => vec![CloudflareMint::describe(&name, &lowered)?],
		false => vec![
			CloudflareMint::converge(&cx.caller, &name, &lowered, &params)
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
				for (entity, action) in Self::declared_actions(world) {
					// the action's OWN stack: a route's verbs may sit outside
					// every `<Stack>`, resolving the addresses above them
					let stack = world.with_state::<StackQuery, _>(|stacks| {
						stacks.resolve(entity)
					});
					lowered = lowered.lower_action(&stack, action)?;
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

	/// Every entity carrying one of the actions [`DeployerToken`] lowers, with
	/// the action's name. Driven by that table through the type registry rather
	/// than by a second list of queries, so adding an action to the table is the
	/// whole change; a name the registry does not know is a Cloudflare feature
	/// this binary was not built with, which is why absence is skipped rather
	/// than refused (`declares_every_lowered_action` holds the table honest).
	fn declared_actions(world: &mut World) -> Vec<(Entity, &'static str)> {
		let registry = world.resource::<AppTypeRegistry>().clone();
		DeployerToken::action_names()
			.filter_map(|action| {
				let reflect_component = {
					let registry = registry.read();
					reflect_ext::registration_by_name(&registry, action)?
						.data::<ReflectComponent>()
						.cloned()?
				};
				let component = reflect_component.register_component(world);
				QueryBuilder::<Entity>::new(world)
					.with_id(component)
					.build()
					.iter(world)
					.map(|entity| (entity, action))
					.collect::<Vec<_>>()
					.xmap(Some)
			})
			.flatten()
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

	/// The dry run's answer: the token, every group with what asked for it, and
	/// the policies a mint would post, pretty printed so it reads and pipes.
	///
	/// An entry declaring no account still gets the whole list, and the one line
	/// it is missing instead of a token: what an entry ASKS FOR is worth reading
	/// whether or not anybody has said where its token would live.
	fn describe(name: &str, lowered: &DeployerToken) -> Result<String> {
		let home = match lowered.account() {
			Ok(account) => format!("account {account}"),
			Err(err) => format!("NOT MINTABLE: {err}"),
		};
		format!(
			"token {name}\n{home}\n\n{lowered}\npolicies\n{}\n",
			serde_json::to_string_pretty(&lowered.to_json())?
		)
		.xok()
	}

	/// Converge the token: mint one when the account holds none by this name,
	/// when the one it holds grants something else, when the document does not
	/// hold it, or on `--rotate`. The new value is sealed only once proven, and
	/// the token it replaces deleted only once sealed.
	async fn converge(
		caller: &AsyncEntity,
		name: &str,
		lowered: &DeployerToken,
		params: &CloudflareMintParams,
	) -> Result<String> {
		let account = lowered.account()?.clone();
		let handle = SecretsHandle::resolve(caller, None).await?;
		let identity = AgeIdentityFile::require()?;
		let mut document = handle.read_or_new().await?;
		let held = document.open(&identity).ok().and_then(|opened| {
			opened.get(Self::RECORD).map(|secret| secret.value.clone())
		});
		let existing = Self::find_token(&account, name).await?;
		// current means all three: the account's token grants what the
		// declarations ask for, it is active, and the document holds THAT
		// token rather than another of the same name
		let current = match existing.as_ref() {
			Some(held_token) => {
				held_token.active
					&& DeployerToken::fingerprint_of(&held_token.policies)
						== lowered.fingerprint()
					&& Self::holds(&account, held.as_deref(), &held_token.id)
						.await
			}
			None => false,
		};
		if current && !params.rotate {
			return format!(
				"token {name} ({}) matches the declarations and is sealed in \
				{}, `--rotate` mints another",
				existing.map(|held_token| held_token.id).unwrap_or_default(),
				handle.describe()
			)
			.xok();
		}
		let (id, value) =
			Self::create_token(&account, name, lowered.to_json()).await?;
		let (url, what) = Self::deploy_proof(lowered)?;
		Self::prove_token(&value, url, what).await?;
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
			rotation: Some(Self::rotation()),
			..default()
		})?;
		handle.write(&document).await?;
		// only now: the token that replaces it is sealed and proven
		let replaced = match existing {
			Some(held_token) if held_token.id != id => {
				Self::delete_token(&account, &held_token.id).await?;
				format!(", replacing {}", held_token.id)
			}
			_ => String::new(),
		};
		format!(
			"token {name} ({id}) minted and sealed in {} (group `{group}`){}",
			handle.describe(),
			replaced
		)
		.xok()
	}

	/// How the token rotates: this verb again, which only the mint token can
	/// run.
	fn rotation() -> SecretRotation {
		SecretRotation::manual(
			"beet cloudflare/mint --rotate\n> with the mint token in the \
			environment, which wins over this document: \
			`CLOUDFLARE_API_TOKEN=.. beet cloudflare/mint --rotate`\n> the \
			mint token holds `Account API Tokens Write` and nothing else, and \
			lives in the password manager rather than in any document: it is \
			the one credential the age key must not open\n> the new token is \
			minted, proven with a read this repo's deploy makes and sealed, \
			then the token it replaces is deleted",
		)
	}

	/// The account's token named `name`, [`None`] when it holds none. Paged to
	/// the end: a token past the first page would read as absent, and the verb
	/// would mint a second token of the same name on every run.
	async fn find_token(
		account: &str,
		name: &str,
	) -> Result<Option<HeldToken>> {
		const PER_PAGE: usize = 50;
		for page in 1.. {
			let body = Self::send(
				Request::get(format!(
					"{API_BASE}/accounts/{account}/tokens?per_page={PER_PAGE}&page={page}"
				)),
				"listing the account's api tokens",
			)
			.await?;
			let listed = body["result"]
				.as_array()
				.map(Vec::as_slice)
				.unwrap_or_default();
			if let Some(held) = listed
				.iter()
				.find(|token| token["name"] == name)
				.and_then(HeldToken::parse)
			{
				return Some(held).xok();
			}
			let pages =
				body["result_info"]["total_pages"].as_u64().unwrap_or(1);
			if page >= pages.max(1) {
				return None.xok();
			}
		}
		None.xok()
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

	/// Create the token, answering its id and its value. The value is the only
	/// copy Cloudflare will ever hand over, so nothing here logs the body and a
	/// failure is reported through the answer's `errors` alone.
	async fn create_token(
		account: &str,
		name: &str,
		policies: Value,
	) -> Result<(SmolStr, SmolStr)> {
		let response = Self::authed(Request::post(format!(
			"{API_BASE}/accounts/{account}/tokens"
		)))?
		.with_json_body(&serde_json::json!({
			"name": name,
			"policies": policies,
		}))?
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

	/// Converge one bucket's token: mint and park when the pair is not there,
	/// when the parked access key id is not the account's live token of that
	/// name, or on `--rotate`.
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
		let existing = Self::find_token(&account, &name).await?;
		let current = match (&parked, &existing) {
			((Some(access_key), Some(_)), Some(held_token)) => {
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
		let (id, value) =
			Self::create_token(&account, &name, block.token_policies(stack)?)
				.await?;
		Self::prove_token(
			&value,
			format!("{API_BASE}/accounts/{account}/tokens/verify"),
			"verify itself",
		)
		.await?;
		// the SECRET half first, and the order is the recovery: a crash between
		// the two writes leaves the OLD id beside the new secret, which no
		// longer matches the account's token, so the next run mints again. The
		// other order would leave a pair that looks current and is not.
		store
			.overwrite(
				&secret_ref,
				&R2BucketBlock::derive_secret_key(&value),
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
			.await?;
		// only now: the pair that replaces it is parked
		let replaced = match existing {
			Some(held_token) if held_token.id != id => {
				Self::delete_token(&account, &held_token.id).await?;
				format!(", replacing {}", held_token.id)
			}
			_ => String::new(),
		};
		format!(
			"bucket token {name} ({id}) minted and parked at {} and {}{}",
			store.address(&access_ref),
			store.address(&secret_ref),
			replaced
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
}

impl HeldToken {
	fn parse(listed: &Value) -> Option<Self> {
		Self {
			id: listed["id"].as_str()?.into(),
			policies: listed["policies"].clone(),
			active: listed["status"] == "active",
		}
		.xmap(Some)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Every action the lowering names is a registered component, so the world
	/// walk finds the ones a route declares. A name that resolves to nothing
	/// here is a typo in the table, which would otherwise look exactly like a
	/// Cloudflare feature the binary was built without — hence the gate: the
	/// table spans the wrangler actions and the mail audit, so only a build
	/// carrying both can hold it honest.
	#[cfg(all(feature = "cloudflare_block", feature = "mail"))]
	#[beet_core::test]
	fn declares_every_lowered_action() {
		let world = InfraPlugin.into_world();
		let registry = world.resource::<AppTypeRegistry>().read();
		for action in DeployerToken::action_names() {
			reflect_ext::registration_by_name(&registry, action)
				.unwrap_or_else(|| {
					panic!(
						"`DeployerToken::ACTIONS` names `{action}`, which no \
						type is registered under"
					)
				})
				.data::<ReflectComponent>()
				.unwrap_or_else(|| {
					panic!("`{action}` is registered but not a component")
				});
		}
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
}

//! `<CloudflareDeployToken/>`: the Cloudflare deploy credential, every token
//! the declarations ask for, minted in the one place that holds the group
//! which mints.

use crate::actions::cloudflare_api_ext;
use crate::actions::cloudflare_api_ext::API_BASE;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;

/// `<CloudflareDeployToken/>` — the repo's Cloudflare deploy credential,
/// declared at the entry's root like `<Secrets/>`, and every Cloudflare token
/// its declarations ask for:
///
/// - **the deploy token**: one account-owned token per repo, scoped to exactly
///   the permission groups the declarations need ([`DeployerToken`], lowered
///   from what the stacks render and the Cloudflare actions their routes run),
///   sealed into the credential document as `CLOUDFLARE_API_TOKEN` with the
///   grants it holds in its record's metadata.
/// - **one token per declared `<R2BucketBlock/>`**, scoped to that bucket's
///   objects and nothing else, its S3 pair parked in the stack's own secret
///   store where the compute beside the bucket reads it. Not sealed in any
///   document: it is a runtime credential, not a deploy one.
///
/// Nobody mints either by hand: a deploy checks the sealed grants against the
/// declarations before it writes anything, and refuses naming `--elevated`
/// when they fall short; the elevated deploy ([`DeployGate`]) converges both
/// here.
///
/// ```jsx
/// <CloudflareDeployToken/>
/// <CloudflareDeployToken ttl_days=30 request_ips={["203.0.113.7/32"]}/>
/// ```
///
/// ## Which credential an elevated deploy runs this as
///
/// Creating a token needs `Account API Tokens Write`, which is the one group
/// that can mint a credential wider than itself and therefore the one group a
/// held credential must not have. So the converge does not run as the token it
/// converges but as the **mint token**, which holds that group and nothing
/// else and is kept NOWHERE: the elevated deploy shows where to roll it, asks
/// for the fresh value with echo off, and carries on, the way it asks for the
/// AWS code. A roll kills the value from last time, wherever it was left.
/// Cloudflare has no mfa condition to put on a token and no permissions
/// boundary to cap one with, so the dashboard login a roll needs is the human
/// factor, and an agent with the age identity opens every document and still
/// cannot mint.
///
/// ```text
/// this elevated deploy runs Cloudflare as the mint token, rolled for this run:
///
/// 1. Roll it at https://dash.cloudflare.com/<account>/api-tokens
///    beet-mint -> `...` -> Roll -> Roll token -> Your API Token -> Copy
/// 2. Paste it here (not echoed):
/// ```
///
/// A value in the environment that is not the sealed deploy token is taken
/// instead, the way CI hands one over. With no terminal to ask on (an agent),
/// the deploy is answered with the steps to relay: the command to run in a
/// terminal, then the page's full url and the clicks.
///
/// ## The change the deploy token cannot make
///
/// Some declared types are only REFRESHED by the deploy token, and written by
/// a group it never holds ([`DeployerToken::elevated`]): an R2 bucket's
/// configuration, since the group that writes its lock also lifts it, over
/// every bucket in the account. A plain deploy that would change one is
/// refused before it writes anything. The elevated deploy mints
/// `<repo>-elevated`, holding the deploy token's groups and the elevated ones,
/// proves it, runs the deploy with it in `CLOUDFLARE_API_TOKEN`, and deletes it
/// once the deploy exits, however it exits. It is never sealed, a token a crash
/// leaves behind is deleted by the next run, and every one lapses within the
/// hour regardless.
///
/// ## Why derived rather than hand-made, and why not an apply
///
/// Against a hand-made token: a credential written by hand drifts from the
/// declarations the moment a block is added, and the drift surfaces as a 403
/// mid-deploy that names nothing useful. Lowered from the render, the token is
/// as narrow as the declarations allow and widens only when they do.
///
/// Against an apply, which CAN mint a token (`cloudflare_account_token` is a
/// bound resource, and the R2 bucket's credential was rendered that way once):
/// an apply that mints needs `Account API Tokens Write` in the credential every
/// deploy of that repo reads, which is exactly the escalation with no boundary
/// to cap it. Minting is rare and deploying is constant, so they hold
/// different credentials, and the rare one is the only one that can mint.
///
/// ## What converges
///
/// - the token, named `<repo>-deploy` after the workspace directory (a document
///   holds one token however many stacks it deploys), created unless the
///   account holds exactly one token of that name, active, granting what the
///   declarations ask for under the declared terms, and not yet inside its
///   renewal notice.
/// - the record, sealed as `CLOUDFLARE_API_TOKEN` in the document's group with
///   the token's `expires` and its grants. The value is written only after the
///   new token has PROVEN itself with a read the deploy actually makes, and
///   every other token of the name is deleted only after that: a failure
///   anywhere leaves the previous credential in place, and a token that never
///   proved itself is deleted on the spot rather than left live beside it. A
///   current token's record is resealed, value unchanged, when its note, roll
///   or grants are not what this writes now.
/// - a bucket's token, minted unless the account holds exactly one token of its
///   name, its pair is parked, and the parked access key id is that token's, or
///   on `--roll`. The parked access key id IS the token id, so that
///   comparison costs no call of its own.
///
/// What it deliberately does not do is delete a token it did not make. A
/// hand-made token is user-owned, which `Account API Tokens Write` cannot touch
/// at all, so it is the operator's to revoke in the dashboard.
///
/// ## The two levers that are not a boundary
///
/// Cloudflare caps a token by nothing but its scope, so the deploy token may
/// also be held to two terms that narrow the window a leaked value is good for.
/// Both are opt-in, since each costs an elevated deploy by hand:
///
/// - **a lifetime**, `ttl_days`: the token stops authenticating on a day, so a
///   leak has a deadline however it leaked. The sealed record carries that day
///   as its `expires`, every launch that loads it warns in the fortnight before
///   ([`SecretRecord::EXPIRY_NOTICE`]), a plain deploy notes it and refuses
///   once it has passed, and an elevated deploy renews inside the same
///   fortnight. Renewal is a fresh token, never an extended one: an extension
///   would keep a leaked value alive with it. Absent, the token lasts until
///   rolled, which suits a token as narrow as a zone's records and settings:
///   its worst case is rewritten records the next apply restores.
/// - **an address filter**, `request_ips`: the CIDRs the token may be used
///   from, any address when empty. Worth declaring only where every deploy
///   leaves from fixed addresses (a self-hosted CI runner, a static egress
///   box); a laptop's address changes with the network it is on, and each
///   change would cost an elevated deploy.
///
/// A bucket's token is held to neither: the box beside the bucket uses it
/// nightly and cannot renew it, and the bucket's lock is what bounds its
/// damage.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct CloudflareDeployToken {
	/// The days a minted deploy token authenticates for, to the midnight UTC
	/// it lands on, or until rolled when absent. At least twice the renewal
	/// notice, so a token spends most of its life quiet.
	pub ttl_days: Option<u32>,
	/// The CIDRs the deploy token may be used from (a bare address is its
	/// `/32` or `/128`), any address when empty.
	pub request_ips: Vec<SmolStr>,
	/// The mint token's name on the account's api tokens page, which the
	/// steps for rolling it name.
	pub mint_token: SmolStr,
}

impl Default for CloudflareDeployToken {
	fn default() -> Self {
		Self {
			ttl_days: None,
			request_ips: Vec::new(),
			mint_token: "beet-mint".into(),
		}
	}
}

impl DeployCredentialProvider for CloudflareDeployToken {
	fn id(&self) -> &'static str { "cloudflare" }

	fn human_factor(&self) -> &'static str {
		"the Cloudflare dashboard login, to roll the mint token"
	}

	fn records(&self) -> &'static [&'static str] { &[Self::RECORD] }

	fn guards(&self, declared: &str) -> bool {
		!DeployerToken::elevated_for(declared).is_empty()
	}

	fn protects(
		&self,
		change: &tofu::PlannedChange,
		_stack: &ResolvedStack,
	) -> Option<String> {
		let needs = DeployerToken::elevated_for(&change.resource_type);
		(!needs.is_empty()).then(|| {
			format!(
				"this deploy would {} `{}`, which the deploy token only reads: \
				writing it needs {}, which reaches what no deploy rebuilds",
				change.action,
				change.address,
				needs
					.iter()
					.map(ToString::to_string)
					.collect::<Vec<_>>()
					.join(", ")
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
		let this = self.clone();
		Box::pin(async move { this.describe_of(&caller).await })
	}
}

impl CloudflareDeployToken {
	/// The record the token is sealed under, which is the variable every
	/// Cloudflare consumer reads: the tofu provider, `wrangler` and the zone
	/// verbs all take it from the environment, so what the document holds is
	/// what they get.
	pub(crate) const RECORD: &'static str = "CLOUDFLARE_API_TOKEN";

	/// The group a first converge seals into; a record already sealed stays
	/// in its own.
	const DEFAULT_GROUP: &'static str = SecretsDocument::DEFAULT_GROUP;

	/// The terms a token minted at `now` is held to.
	fn terms(&self, now: Timestamp) -> Result<TokenTerms> {
		TokenTerms::new(self.ttl_days, &self.request_ips, now)
	}

	/// Where the sealed token stands against what this launch declares: its
	/// grants against the record's, and its lifetime against the notice.
	async fn status_of(
		&self,
		caller: &AsyncEntity,
	) -> Result<CredentialStatus> {
		let Some(lowered) = Self::lower(caller).await? else {
			return CredentialStatus::not_needed().xok();
		};
		let now = Timestamp::now();
		let grants =
			self.grants(&lowered, &Self::bucket_names(caller).await?)?;
		let held = DeployCredential::sealed(caller, Self::RECORD).await;
		let mut status = CredentialStatus::compare(
			self.id(),
			Self::RECORD,
			&grants,
			held.as_ref(),
			None,
		);
		match held.map(|held| held.record.expiry(now)) {
			Some(SecretExpiry::Expired { at }) => status.stale.push(format!(
				"{}: the sealed token stopped authenticating at {}",
				self.id(),
				at.format_iso8601_secs()
			)),
			Some(SecretExpiry::Expiring { at }) => status.notes.push(format!(
				"{}: the sealed token stops authenticating at {}, so the next \
				elevated deploy renews it",
				self.id(),
				at.format_iso8601_secs()
			)),
			_ => {}
		}
		status.xok()
	}

	/// The grants this launch's declarations ask the credential for, one line
	/// each: every group over every resource it reaches, the token's terms,
	/// and each bucket's own token. What the sealed record carries in its
	/// metadata once converged, and what a plain deploy compares against.
	fn grants(
		&self,
		lowered: &DeployerToken,
		buckets: &[String],
	) -> Result<Vec<String>> {
		let terms = self.terms(Timestamp::now())?;
		let mut grants = lowered.grant_lines();
		grants.push(match self.ttl_days {
			Some(days) => format!("lifetime: {days} days"),
			None => "lifetime: until rolled".into(),
		});
		grants.push(match terms.request_ips.is_empty() {
			true => "usable from: any address".into(),
			false => format!("usable from: {}", terms.request_ips.join(", ")),
		});
		grants.extend(
			buckets
				.iter()
				.map(|bucket| format!("bucket token: {bucket}")),
		);
		grants.sort();
		grants.xok()
	}

	/// Ask for the mint token, converge the deploy token and every bucket's
	/// own, and when `ask.run`, mint the elevated token the deploy runs as.
	async fn elevate_with(
		&self,
		caller: &AsyncEntity,
		ask: ElevationAsk,
	) -> Result<CredentialElevation> {
		let Some(lowered) = Self::lower(caller).await? else {
			bevybail!(
				"this launch declares nothing at Cloudflare, so there is no token \
				to elevate"
			);
		};
		if let Some(askers) = lowered.escalating() {
			warn!(
				"the lowered token carries `{}`, so this repo's deploy credential \
				can mint any token the account holds: asked for by {}. A \
				bucket's own token is the only thing that needs it, and an apply \
				that mints one is what puts it here",
				TokenPermission::API_TOKENS_WRITE,
				askers
					.iter()
					.map(SmolStr::as_str)
					.collect::<Vec<_>>()
					.join(", ")
			);
		}
		let terms = self.terms(Timestamp::now())?;
		let account = lowered.account()?.clone();
		let steps =
			MintSteps::new(&account, &self.mint_token, ask.relay.clone());
		let grants =
			self.grants(&lowered, &Self::bucket_names(caller).await?)?;
		let (line, mint) = Self::converge(
			caller,
			&Self::token_name()?,
			&lowered,
			&terms,
			&steps,
			ask.roll,
			&grants,
		)
		.await?;
		let mut report = vec![line];
		report.extend(Self::buckets(caller, Some(&mint), ask.roll).await?);
		let mut elevation = CredentialElevation {
			report,
			..default()
		};
		if ask.run {
			let (line, env, cleanup) =
				Self::mint_elevated(mint, &lowered, &terms).await?;
			elevation.report.push(line);
			elevation.env = env;
			elevation.cleanup = Some(cleanup);
		}
		elevation.xok()
	}

	/// The dry run's answer, needing no credential: the token, its terms,
	/// every group with what asked for it and the body a converge would post,
	/// then each bucket's token.
	async fn describe_of(&self, caller: &AsyncEntity) -> Result<String> {
		let Some(lowered) = Self::lower(caller).await? else {
			return "cloudflare: this launch declares nothing at Cloudflare\n"
				.to_string()
				.xok();
		};
		let mut out = Self::describe(
			&Self::token_name()?,
			&lowered,
			&self.terms(Timestamp::now())?,
		)?;
		for line in Self::buckets(caller, None, false).await? {
			out.push_str(&format!("{line}\n"));
		}
		out.xok()
	}

	/// Lower every stack this launch declares, at the launch's stage and at
	/// `prod` ([`DeployCredential::render_stages`]), and every Cloudflare
	/// action its routes carry, in one world pass, so a stack that cannot
	/// render fails here rather than at the account. `None` when this launch
	/// asks nothing of Cloudflare.
	async fn lower(caller: &AsyncEntity) -> Result<Option<DeployerToken>> {
		caller
			.with_world(|world, _| {
				let mut lowered = DeployerToken::default();
				for (stack, _deployment, config) in
					DeployCredential::render_stages(world)?.iter()
				{
					lowered = lowered.lower(stack, config)?;
				}
				for (entity, access) in Self::declared_access(world) {
					// the action's OWN stack: a route's verbs may sit outside
					// every `<Stack>`, resolving the addresses above them
					let stack = world.with_state::<StackQuery, _>(|stacks| {
						stacks.resolve(entity)
					});
					let bucket = access.bucket(world.entity(entity));
					lowered = lowered.lower_access(
						&stack,
						&access,
						bucket.as_deref(),
					)?;
				}
				match lowered.is_empty() {
					true => None,
					false => Some(lowered),
				}
				.xok()
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
	/// ([`AwsDeployer::user_name`] names the AWS deployer the same way).
	fn token_name() -> Result<String> {
		format!("{}-deploy", Self::repo_name()?).xok()
	}

	/// The token an elevated run holds for its one command, named beside the
	/// deploy token so the account's api tokens page reads as one repo's pair.
	fn elevated_name() -> Result<String> {
		format!("{}-elevated", Self::repo_name()?).xok()
	}

	/// The workspace directory, kebab-cased: the one name a repo has.
	fn repo_name() -> Result<String> {
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
		name.xok()
	}

	/// The token's half of the dry run: the token, its terms, every group with
	/// what asked for it, and the body a converge would post, pretty printed so
	/// it reads and pipes.
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
		let elevated = match lowered.elevated().is_empty() {
			true => String::new(),
			false => "an elevated deploy also holds each `elevated:` group, \
			for the run that changes what it guards\n"
				.to_string(),
		};
		format!(
			"token {name}\n{home}\n{terms}\n\n{lowered}{elevated}\nbody\n{}\n",
			serde_json::to_string_pretty(&terms.body(name, lowered.to_json()))?,
		)
		.xok()
	}

	/// Converge the token: mint one unless the account holds exactly one of
	/// this name that grants what the declarations ask for under the declared
	/// terms, is active and not yet due, and is the one the document holds; or
	/// on `roll`. The new value is sealed with its `grants` only once proven,
	/// and every other token of the name deleted only once sealed. Answers the
	/// report and the mint token the run acts as, which the buckets are
	/// converged with next.
	async fn converge(
		caller: &AsyncEntity,
		name: &str,
		lowered: &DeployerToken,
		terms: &TokenTerms,
		steps: &MintSteps,
		roll: bool,
		grants: &[String],
	) -> Result<(String, MintToken)> {
		let account = lowered.account()?.clone();
		let handle = SecretsHandle::resolve(caller, None).await?;
		let identity = AgeIdentityFile::require()?;
		let mut document = handle.read_or_new().await?;
		let held = document
			.open(&identity)
			.ok()
			.and_then(|opened| opened.get(Self::RECORD).cloned());
		let held_value = held.as_ref().map(|secret| secret.value.as_str());
		// a record stays in the group it was sealed in
		let group = held
			.as_ref()
			.map(|secret| secret.group.as_str())
			.unwrap_or(Self::DEFAULT_GROUP);
		let (mint, existing) =
			MintToken::open(steps, held_value, &account, name).await?;
		// a second token of the name is never current: it is a mint a failure
		// interrupted, and minting again is what deletes it
		let current = match existing.as_slice() {
			[held_token] => {
				held_token.active
					&& DeployerToken::fingerprint_of(&held_token.policies)
						== lowered.fingerprint()
					&& terms.kept_by(held_token, Timestamp::now())
					&& Self::holds(&account, held_value, &held_token.id).await
			}
			_ => false,
		};
		if current && !roll {
			let token = &existing[0];
			let restated = held.as_ref().and_then(|secret| {
				DeployCredential::restated(
					secret,
					group,
					Self::record(name, &token.id, token.expires_on, grants),
				)
				.map(|record| (secret, record))
			});
			let resealed = match restated {
				Some((secret, record)) => {
					document.set(
						&identity,
						group,
						Self::RECORD,
						&secret.value,
						record,
					)?;
					handle.write(&document).await?;
					", its note, roll and grants resealed"
				}
				None => "",
			};
			let line = format!(
				"token {name} ({}) matches the declarations, {}, and is sealed \
				in {}{resealed}",
				token.id,
				TokenTerms::lifetime(token.expires_on),
				handle.describe()
			);
			return (line, mint).xok();
		}
		let (id, value) = mint
			.create_token(&account, terms.body(name, lowered.to_json()))
			.await?;
		let (url, what) = Self::deploy_proof(lowered)?;
		mint.prove_or_discard(&account, &id, &value, url, what)
			.await?;
		document.set(
			&identity,
			group,
			Self::RECORD,
			&value,
			Self::record(name, &id, terms.expires_on, grants),
		)?;
		handle.write(&document).await?;
		// only now: the token that replaces them is sealed and proven
		let replaced = mint.delete_others(&account, &existing, &id).await?;
		let line = format!(
			"token {name} ({id}) minted, {}, and sealed in {} (group \
			`{group}`){replaced}",
			TokenTerms::lifetime(terms.expires_on),
			handle.describe(),
		);
		(line, mint).xok()
	}

	/// The record the deploy token `id` named `name` is sealed under, expiring
	/// at `expires` and holding `grants`: what it is, how it rolls, and what a
	/// plain deploy compares the declarations against.
	fn record(
		name: &str,
		id: &str,
		expires: Option<Timestamp>,
		grants: &[String],
	) -> SecretRecord {
		SecretRecord {
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
			expires,
			roll: Some(SecretRoll::Elevated),
			metadata: BTreeMap::from([(
				DeployCredential::GRANTS.into(),
				grants.join("\n").into(),
			)]),
			..default()
		}
	}

	/// Mint `<repo>-elevated` with `mint`: the deploy token's groups and the
	/// elevated ones, proven, held for one deploy. Answers the report line,
	/// the variable the deploy runs with, and its deletion, which owns the mint
	/// token so it runs however the deploy exits; a failed deletion names the
	/// instant the token lapses on its own.
	async fn mint_elevated(
		mint: MintToken,
		lowered: &DeployerToken,
		terms: &TokenTerms,
	) -> Result<(
		String,
		Vec<(SmolStr, SmolStr)>,
		SendBoxedFuture<Result<String>>,
	)> {
		let account = lowered.account()?.clone();
		let name = Self::elevated_name()?;
		let elevated = lowered.elevate();
		let terms = terms.elevated(Timestamp::now());
		// a token a crash left behind is never the one this run holds
		let leftover = mint.find_tokens(&account, &name).await?;
		mint.delete_others(&account, &leftover, "").await?;
		let (id, value) = mint
			.create_token(&account, terms.body(&name, elevated.to_json()))
			.await?;
		let (url, what) = Self::deploy_proof(&elevated)?;
		mint.prove_or_discard(&account, &id, &value, url, what)
			.await?;
		let line = format!(
			"{name} ({id}) minted for this deploy, which also holds {}",
			lowered
				.elevated()
				.keys()
				.map(TokenPermission::name)
				.collect::<Vec<_>>()
				.join(", ")
		);
		let lapses = terms
			.expires_on
			.map(|at| at.format_iso8601_secs())
			.unwrap_or_default();
		let cleanup: SendBoxedFuture<Result<String>> = Box::pin(async move {
			match mint.delete_token(&account, &id).await {
				Ok(()) => format!("{name} ({id}) deleted").xok(),
				Err(err) => bevybail!(
					"deleting {name} ({id}) failed, so it lapses at {lapses} \
					unless deleted in the dashboard first: {err}"
				),
			}
		});
		(line, vec![(SmolStr::new(Self::RECORD), value)], cleanup).xok()
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

	/// Converge the token of every `<R2BucketBlock/>` this launch declares as
	/// `mint`, or describe what that would do when there is none (a dry run).
	/// One report line per bucket, none at all for a launch that declares
	/// none.
	///
	/// A bucket's token is not a deploy credential and is never sealed in a
	/// document: its S3 pair is parked in the stack's own secret store, where
	/// the compute beside the bucket reads it under its own grants.
	#[cfg(feature = "cloudflare_dns")]
	async fn buckets(
		caller: &AsyncEntity,
		mint: Option<&MintToken>,
		roll: bool,
	) -> Result<Vec<String>> {
		let mut report = Vec::new();
		let declared = caller
			.with_world(|world, _| Self::declared_buckets(world))
			.await??;
		for (block, stack, store) in declared {
			report.push(match mint {
				None => format!(
					"bucket token {} over {}, parked at {} and {}",
					block.token_name(&stack),
					block.bucket_name(&stack),
					store.address(&block.access_key_secret()),
					store.address(&block.secret_key_secret()),
				),
				Some(mint) => {
					Self::converge_bucket(&block, &stack, &store, mint, roll)
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
	/// key id is that token's; or on `--roll`.
	///
	/// The parked access key id IS the token id, so the comparison needs no
	/// call beyond the listing every converge makes anyway.
	#[cfg(feature = "cloudflare_dns")]
	async fn converge_bucket(
		block: &R2BucketBlock,
		stack: &ResolvedStack,
		store: &SecretStore,
		mint: &MintToken,
		roll: bool,
	) -> Result<String> {
		let account = stack.cloudflare_account()?.id().to_string();
		let name = block.token_name(stack);
		let (access_ref, secret_ref) =
			(block.access_key_secret(), block.secret_key_secret());
		let parked =
			(store.get(&access_ref).await?, store.get(&secret_ref).await?);
		let existing = mint.find_tokens(&account, &name).await?;
		let current = match (&parked, existing.as_slice()) {
			((Some(access_key), Some(_)), [held_token]) => {
				held_token.active && held_token.id == *access_key
			}
			_ => false,
		};
		if current && !roll {
			let restated = Self::restate_parked(block, store).await?;
			return format!(
				"bucket token {name} is parked at {} and {}{restated}",
				store.address(&access_ref),
				store.address(&secret_ref)
			)
			.xok();
		}
		let (id, value) = mint
			.create_token(
				&account,
				serde_json::json!({
					"name": name,
					"policies": block.token_policies(stack)?,
				}),
			)
			.await?;
		mint.prove_or_discard(
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
				Some(Self::bucket_roll()),
			)
			.await?;
		store
			.overwrite(
				&access_ref,
				&id,
				Some(&block.access_key_note()),
				Some(Self::bucket_roll()),
			)
			.await
			.map_err(|err| {
				bevyhow!(
					"parked the secret half of {name} but not its access key \
					id, so the parked pair authenticates nowhere until the \
					next elevated deploy: {err}"
				)
			})?;
		// only now: the pair that replaces them is parked
		let replaced = mint.delete_others(&account, &existing, &id).await?;
		format!(
			"bucket token {name} ({id}) minted and parked at {} and {}{replaced}",
			store.address(&access_ref),
			store.address(&secret_ref),
		)
		.xok()
	}

	/// Re-park a current pair whose stored note or roll is not what a converge
	/// parks now (a roll naming the `cloudflare_account_token` an apply once
	/// rendered), values unchanged, answering the report's clause.
	#[cfg(feature = "cloudflare_dns")]
	async fn restate_parked(
		block: &R2BucketBlock,
		store: &SecretStore,
	) -> Result<&'static str> {
		let parked = store.list().await?;
		let mut restated = false;
		for (secret, note) in [
			(block.secret_key_secret(), block.secret_key_note()),
			(block.access_key_secret(), block.access_key_note()),
		] {
			let stale = parked
				.iter()
				.find(|entry| entry.secret == secret)
				.is_some_and(|entry| {
					entry.roll != Some(Self::bucket_roll())
						|| entry.note.as_deref() != Some(note.as_str())
				});
			if !stale {
				continue;
			}
			if let Some(value) = store.get(&secret).await? {
				store
					.overwrite(
						&secret,
						&value,
						Some(&note),
						Some(Self::bucket_roll()),
					)
					.await?;
				restated = true;
			}
		}
		match restated {
			true => ", its note and roll re-parked",
			false => "",
		}
		.xok()
	}

	/// A launch built without the Cloudflare bindings declares no bucket, so
	/// there is nothing to converge.
	#[cfg(not(feature = "cloudflare_dns"))]
	async fn buckets(
		_caller: &AsyncEntity,
		_mint: Option<&MintToken>,
		_roll: bool,
	) -> Result<Vec<String>> {
		Vec::new().xok()
	}

	/// The name of each declared bucket's own token, which the grants list so
	/// a bucket declared since the last converge makes the credential stale.
	#[cfg(feature = "cloudflare_dns")]
	async fn bucket_names(caller: &AsyncEntity) -> Result<Vec<String>> {
		caller
			.with_world(|world, _| Self::declared_buckets(world))
			.await??
			.into_iter()
			.map(|(block, stack, _)| block.token_name(&stack))
			.collect::<Vec<_>>()
			.xok()
	}

	/// A launch built without the Cloudflare bindings declares no bucket.
	#[cfg(not(feature = "cloudflare_dns"))]
	async fn bucket_names(_caller: &AsyncEntity) -> Result<Vec<String>> {
		Vec::new().xok()
	}

	/// How a bucket's parked pair rolls: an elevated deploy with `--roll`,
	/// which replaces the token and re-parks both halves. The S3 secret is the
	/// SHA-256 of the token value, so neither half survives the other.
	#[cfg(feature = "cloudflare_dns")]
	fn bucket_roll() -> SecretRoll { SecretRoll::Elevated }
}

/// The mint token one run acts as: taken for the run and written nowhere.
/// Every call that reads or edits the account's api tokens goes through it,
/// so none can fall back to the deploy token the document puts in the
/// environment.
#[derive(Clone)]
struct MintToken(SmolStr);

impl MintToken {
	/// How many values a run takes before giving up, so a mis-paste costs a
	/// re-prompt rather than a rerun.
	const ATTEMPTS: usize = 3;

	/// The mint token for this run and the account's tokens named `name`,
	/// listed with it, which is the first call that proves it.
	///
	/// A value passed in the environment is taken when it is not the deploy
	/// token the document sealed (`held`), the way CI hands one over; else the
	/// operator is shown the steps that roll it and asked to paste it. A refused
	/// value is asked for again, and without a terminal to ask on, the steps
	/// are the error, naming the command to run in one.
	async fn open(
		steps: &MintSteps,
		held: Option<&str>,
		account: &str,
		name: &str,
	) -> Result<(Self, Vec<HeldToken>)> {
		let mut passed = cloudflare_api_ext::token()
			.ok()
			.filter(|token| Some(token.as_str()) != held);
		let mut attempt = 1;
		loop {
			let mint = match passed.take() {
				Some(token) => Self(token),
				None => Self(steps.prompt()?.into()),
			};
			match mint.find_tokens(account, name).await {
				Ok(found) => return (mint, found).xok(),
				Err(err) if attempt < Self::ATTEMPTS => {
					match MintSteps::refused(&err) {
						Some(refusal) => {
							warn!(
								"Cloudflare refused that token ({}), so paste it \
							again",
								refusal.messages()
							);
							attempt += 1;
						}
						None => return Err(err),
					}
				}
				Err(err) => return Err(steps.explain(err)),
			}
		}
	}

	/// An api call as this token.
	async fn send(&self, request: Request, what: &str) -> Result<Value> {
		cloudflare_api_ext::send(request.with_auth_bearer(&self.0), what).await
	}

	/// Every token of the account named `name`, empty when it holds none.
	/// Paged to the end: a token past the first page would read as absent,
	/// and the verb would mint a second token of the same name on every run.
	async fn find_tokens(
		&self,
		account: &str,
		name: &str,
	) -> Result<Vec<HeldToken>> {
		const PER_PAGE: usize = 50;
		let mut found = Vec::new();
		for page in 1.. {
			let body = self
				.send(
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

	/// Create the token `body` describes (its `name`, `policies` and any
	/// terms), answering its id and its value. The value is the only copy
	/// Cloudflare will ever hand over, so nothing here logs the answer and a
	/// failure is reported through its `errors` alone.
	async fn create_token(
		&self,
		account: &str,
		body: Value,
	) -> Result<(SmolStr, SmolStr)> {
		let name = body["name"].as_str().unwrap_or_default().to_string();
		let response =
			Request::post(format!("{API_BASE}/accounts/{account}/tokens"))
				.with_auth_bearer(&self.0)
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
				"minting the token failed: {status} - {}",
				cloudflare_api_ext::error_messages(&body),
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

	/// [`CloudflareDeployToken::prove_token`], deleting the new token `id` when it
	/// fails: unproven and unsealed, it would stay live beside the token it was
	/// to replace, under the same name, with a value nobody holds.
	async fn prove_or_discard(
		&self,
		account: &str,
		id: &str,
		value: &str,
		url: String,
		what: &str,
	) -> Result {
		let Err(err) =
			CloudflareDeployToken::prove_token(value, url, what).await
		else {
			return OK;
		};
		match self.delete_token(account, id).await {
			Ok(()) => Err(err),
			Err(delete_err) => bevybail!(
				"{err}\nand deleting the unproven token {id} failed too, so \
				delete it in the dashboard: {delete_err}"
			),
		}
	}

	/// Delete every token of `existing` but the one just sealed, answering
	/// the report's `, replacing ..` clause.
	async fn delete_others(
		&self,
		account: &str,
		existing: &[HeldToken],
		kept: &str,
	) -> Result<String> {
		let mut replaced = Vec::new();
		for held_token in existing.iter().filter(|token| token.id != kept) {
			self.delete_token(account, &held_token.id).await?;
			replaced.push(held_token.id.as_str());
		}
		match replaced.is_empty() {
			true => String::new(),
			false => format!(", replacing {}", replaced.join(", ")),
		}
		.xok()
	}

	async fn delete_token(&self, account: &str, id: &str) -> Result {
		self.send(
			Request::delete(format!(
				"{API_BASE}/accounts/{account}/tokens/{id}"
			)),
			"deleting an api token",
		)
		.await
		.map(|_| ())
	}
}

/// What the operator does to hand an elevated deploy the mint token: run the
/// deploy elevated in a terminal, roll the token on the account's api tokens
/// page, and paste it when asked. The one place those steps are written: the
/// run itself asks with them, and a run with no terminal to ask on is answered
/// with them. Rolling rather than keeping a copy means a value pasted last
/// time, wherever it was left, has stopped working.
struct MintSteps {
	/// The account whose api tokens page holds the mint token.
	account: SmolStr,
	/// The mint token's name on that page.
	mint_token: SmolStr,
	/// The command the operator runs in a terminal: the deploy, elevated.
	command: String,
}

impl MintSteps {
	/// The steps for `account`'s `mint_token`, run as `command`.
	fn new(account: &str, mint_token: &str, command: String) -> Self {
		Self {
			account: account.into(),
			mint_token: mint_token.into(),
			command,
		}
	}

	/// The account's api tokens page.
	fn url(&self) -> String {
		format!("https://dash.cloudflare.com/{}/api-tokens", self.account)
	}

	/// The clicks from that page to the rolled value on the clipboard.
	fn clicks(&self) -> String {
		format!(
			"{} -> `...` -> Roll -> Roll token -> Your API Token -> Copy",
			self.mint_token
		)
	}

	/// Ask for the mint token on the controlling terminal: how to roll it,
	/// then one line read with echo off. The url stands bare so the terminal
	/// makes it a link. Without a terminal (an agent, a pipe, CI) the steps
	/// are the error.
	fn prompt(&self) -> Result<String> {
		terminal_ext::read_secret_line(&format!(
			"\nthis elevated deploy runs Cloudflare as the mint token, rolled \
			for this run:\n\n\
			1. Roll it at {}\n   {}\n2. Paste it here (not echoed): ",
			self.url(),
			self.clicks()
		))
		.map(|value| value.trim().to_string())
		.map_err(|err| {
			bevyhow!(
				"an elevated deploy asks for the mint token, and {err}\n\n{self}"
			)
		})
	}

	/// The refusal `err` is, when it is a credential Cloudflare refused.
	fn refused(
		err: &BevyError,
	) -> Option<&cloudflare_api_ext::CloudflareApiError> {
		err.downcast_ref::<cloudflare_api_ext::CloudflareApiError>()
			.filter(|refusal| refusal.refused_credential())
	}

	/// `err` answered with these steps when it is a refused credential, else
	/// as it is.
	fn explain(&self, err: BevyError) -> BevyError {
		match Self::refused(&err) {
			Some(refusal) => bevyhow!(
				"an elevated deploy runs Cloudflare as the mint token, and \
				Cloudflare refused that one ({}): {err}\n\n{self}",
				refusal.messages()
			),
			None => err,
		}
	}
}

/// The steps as an operator reads them relayed, every link whole.
impl core::fmt::Display for MintSteps {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		write!(
			formatter,
			"steps:\n\n1. Run this in a terminal\n\t- `{}`\n2. When it asks for the mint token, roll it and paste it there\n\t- [Cloudflare Tokens Page]({}) -> {}",
			self.command,
			self.url(),
			self.clicks()
		)
	}
}

/// What the deploy token is held to besides its scope: the instant it stops
/// authenticating and the addresses it may be used from.
#[derive(Debug, Clone, PartialEq)]
struct TokenTerms {
	/// Midnight UTC `ttl_days` after the mint, [`None`] for a token that lasts
	/// until rolled.
	expires_on: Option<Timestamp>,
	/// Normalized CIDRs, sorted, empty for any address.
	request_ips: Vec<SmolStr>,
}

impl TokenTerms {
	const SECS_PER_DAY: u64 = 24 * 60 * 60;

	/// How long an elevated token authenticates for: a backstop for the delete
	/// a crash would skip, since the run deletes it as soon as the route
	/// exits, and long enough for the longest deploy (a mail box replaced and
	/// provisioned).
	const ELEVATED_LIFETIME: Duration = Duration::from_secs(60 * 60);

	/// The terms an elevated token minted at `now` is held to: these
	/// addresses, for [`ELEVATED_LIFETIME`](Self::ELEVATED_LIFETIME), to the
	/// second.
	fn elevated(&self, now: Timestamp) -> Self {
		Self {
			expires_on: Some(Timestamp::from_secs(
				(now + Self::ELEVATED_LIFETIME).secs(),
			)),
			request_ips: self.request_ips.clone(),
		}
	}

	/// The terms a token minted at `now` is held to, refusing a lifetime the
	/// renewal notice would swallow and an address that is not one.
	fn new(
		ttl_days: Option<u32>,
		request_ips: &[SmolStr],
		now: Timestamp,
	) -> Result<Self> {
		let notice_days =
			SecretRecord::EXPIRY_NOTICE.as_secs() / Self::SECS_PER_DAY;
		if let Some(ttl_days) = ttl_days
			&& u64::from(ttl_days) < notice_days * 2
		{
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
		Self {
			expires_on: ttl_days.map(|ttl_days| {
				let lifetime = Duration::from_secs(
					u64::from(ttl_days) * Self::SECS_PER_DAY,
				);
				Date::from(now + lifetime).timestamp()
			}),
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

	/// Whether `held` was minted under these terms and is not yet due: under a
	/// lifetime it expires after the renewal notice and no later than a token
	/// minted now would, without one it never expires, and either way it is
	/// usable from exactly the declared addresses.
	fn kept_by(&self, held: &HeldToken, now: Timestamp) -> bool {
		let lasts = match (self.expires_on, held.expires_on) {
			(None, None) => true,
			(Some(latest), Some(at)) => {
				at > now + SecretRecord::EXPIRY_NOTICE && at <= latest
			}
			_ => false,
		};
		lasts && held.request_ips == self.request_ips
	}

	/// How long a token expiring at `expires_on` lasts, for a report line.
	fn lifetime(expires_on: Option<Timestamp>) -> String {
		match expires_on {
			Some(at) => format!("expires {}", Date::from(at)),
			None => "lasts until rolled".into(),
		}
	}

	/// The create body of a token named `name` granting `policies` under
	/// these terms.
	fn body(&self, name: &str, policies: Value) -> Value {
		let mut body = serde_json::json!({
			"name": name,
			"policies": policies,
		});
		if let Some(expires_on) = self.expires_on {
			body["expires_on"] = expires_on.format_iso8601_secs().into();
		}
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
		write!(formatter, "{}", Self::lifetime(self.expires_on))?;
		if self.expires_on.is_some() {
			write!(
				formatter,
				", renewed by a mint inside its last {} days",
				SecretRecord::EXPIRY_NOTICE.as_secs() / Self::SECS_PER_DAY
			)?;
		}
		write!(
			formatter,
			"\nusable from {}",
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
	use beet_net::prelude::*;

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
		let mut found =
			super::CloudflareDeployToken::declared_access(&mut world)
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
		let declared =
			CloudflareDeployToken::declared_buckets(&mut world).unwrap();
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
		CloudflareDeployToken::token_name()
			.unwrap()
			.xpect_eq("beet-deploy");
	}
	/// The steps an operator is relayed to run an elevated deploy, rendered
	/// from the account and the deploy's own command, with every link whole,
	/// and a refused credential answered with them.
	#[beet_core::test]
	fn renders_the_operator_steps() {
		let steps = super::MintSteps::new(
			"74aba4da669f57fc6fc3e63ebcfcff26",
			"beet-mint",
			"just cli mail/deploy --stage=prod --elevated".into(),
		);
		steps.to_string().xpect_eq(
			"steps:\n\n1. Run this in a terminal\n\t- `just cli mail/deploy --stage=prod --elevated`\n2. When it asks for the mint token, roll it and paste it there\n\t- [Cloudflare Tokens Page](https://dash.cloudflare.com/74aba4da669f57fc6fc3e63ebcfcff26/api-tokens) -> beet-mint -> `...` -> Roll -> Roll token -> Your API Token -> Copy",
		);
		steps
			.explain(
				super::cloudflare_api_ext::CloudflareApiError {
					what: "listing the account's api tokens".into(),
					status: StatusCode::FORBIDDEN,
					body: "{}".into(),
				}
				.into(),
			)
			.to_string()
			.xpect_contains("403 Forbidden")
			.xpect_contains("1. Run this in a terminal");
	}

	/// A current token's record is resealed only when what is sealed is not
	/// what a converge writes now, and never with a new `modified`: a reseal
	/// on every run would rewrite the committed document each time. Grants
	/// recorded before the declarations changed are stale like an old roll.
	#[beet_core::test]
	fn restates_only_a_stale_record() {
		let grants = vec!["Zone > DNS Write on beet.org".to_string()];
		let today = || {
			super::CloudflareDeployToken::record(
				"beet-deploy",
				"abc",
				None,
				&grants,
			)
		};
		// sealed at `minted_at` as a converge writes it, then changed by `edit`
		let held = |edit: &dyn Fn(SecretRecord) -> SecretRecord| Secret {
			name: "CLOUDFLARE_API_TOKEN".into(),
			group: "default".into(),
			value: "token".into(),
			record: edit(SecretRecord {
				modified: Some(minted_at()),
				..today()
			}),
		};
		let restate = |secret: &Secret, group: &str| {
			DeployCredential::restated(secret, group, today())
		};
		// sealed as a converge writes it now: left alone
		let current = held(&|record| record);
		restate(&current, "default").xpect_none();
		// another group
		restate(&current, "agents").xpect_some();
		// an older roll: resealed, its `modified` kept
		let restated = restate(
			&held(&|record| SecretRecord {
				roll: Some(SecretRoll::manual("in the password manager")),
				..record
			}),
			"default",
		)
		.unwrap();
		restated.modified.xpect_eq(Some(minted_at()));
		restated.roll.xpect_eq(Some(SecretRoll::Elevated));
		// grants from before the declarations changed
		restate(
			&held(&|record| SecretRecord {
				metadata: default(),
				..record
			}),
			"default",
		)
		.unwrap()
		.metadata
		.get(DeployCredential::GRANTS)
		.cloned()
		.xpect_eq(Some(SmolStr::from("Zone > DNS Write on beet.org")));
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

	/// The create body carries a declared lifetime as the midnight it lands
	/// on and an address filter as normalized CIDRs, and neither when none is
	/// declared.
	#[beet_core::test]
	fn the_body_carries_the_terms() {
		let body = super::TokenTerms::new(None, &[], minted_at())
			.unwrap()
			.body("beet-deploy", serde_json::json!([]));
		body.get("expires_on").xpect_none();
		body.get("condition").xpect_none();
		let terms = super::TokenTerms::new(Some(90), &[], minted_at()).unwrap();
		let body = terms.body("beet-deploy", serde_json::json!([]));
		body["expires_on"].xpect_eq("2027-01-05T00:00:00Z");
		body.get("condition").xpect_none();
		let terms = super::TokenTerms::new(
			Some(90),
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
		// an elevated token keeps the addresses and lapses within the hour, to
		// the second, however long the deploy token lasts
		let elevated = terms
			.elevated(minted_at() + Duration::from_millis(1_500))
			.body("beet-elevated", serde_json::json!([]));
		elevated["expires_on"].xpect_eq("2026-10-07T16:00:01Z");
		elevated["condition"]["request_ip"]["in"][0].xpect_eq("203.0.113.7/32");
	}

	/// A lifetime the renewal notice would mostly swallow is refused naming
	/// the floor, as is anything that is not an address.
	#[beet_core::test]
	fn refuses_terms_that_cannot_hold() {
		super::TokenTerms::new(Some(27), &[], minted_at())
			.unwrap_err()
			.to_string()
			.xpect_contains("at least 28");
		super::TokenTerms::new(Some(28), &[], minted_at()).unwrap();
		for cidr in ["beet.org", "203.0.113.7/33", "::1/129", "10.0.0.1/"] {
			super::TokenTerms::new(Some(90), &[cidr.into()], minted_at())
				.unwrap_err();
		}
	}

	/// A held token is kept only while it is minted under the declared terms
	/// and not yet due: one inside its notice, one outliving a shortened
	/// lifetime, one whose lifetime was added or dropped and one usable from
	/// elsewhere are all minted again.
	#[beet_core::test]
	fn keeps_only_a_token_under_the_terms_and_not_yet_due() {
		let terms = super::TokenTerms::new(Some(90), &[], minted_at()).unwrap();
		// the token a mint this afternoon would make, read back
		terms
			.kept_by(&held(Some("2027-01-05T00:00:00Z"), &[]), minted_at())
			.xpect_true();
		// the same token sixty days on, still outside its notice
		let later = minted_at() + days(60);
		super::TokenTerms::new(Some(90), &[], later)
			.unwrap()
			.kept_by(&held(Some("2027-01-05T00:00:00Z"), &[]), later)
			.xpect_true();
		// and eighty days on, inside it
		let due = minted_at() + days(80);
		super::TokenTerms::new(Some(90), &[], due)
			.unwrap()
			.kept_by(&held(Some("2027-01-05T00:00:00Z"), &[]), due)
			.xpect_false();
		// a token that never expires, under a declared lifetime
		terms.kept_by(&held(None, &[]), minted_at()).xpect_false();
		// without one, the token that never expires is the one kept, and a
		// token minted under a lifetime since dropped is minted again
		let lasting = super::TokenTerms::new(None, &[], minted_at()).unwrap();
		lasting.kept_by(&held(None, &[]), minted_at()).xpect_true();
		lasting
			.kept_by(&held(Some("2027-01-05T00:00:00Z"), &[]), minted_at())
			.xpect_false();
		// a lifetime shortened under a token that outlives it
		super::TokenTerms::new(Some(30), &[], minted_at())
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
		super::TokenTerms::new(Some(90), &["203.0.113.7".into()], minted_at())
			.unwrap()
			.kept_by(
				&held(Some("2027-01-05T00:00:00Z"), &["203.0.113.7/32"]),
				minted_at(),
			)
			.xpect_true();
	}
}

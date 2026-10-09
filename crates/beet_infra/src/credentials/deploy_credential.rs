//! A repo's deploy credentials, one per provider, and the seam a deploy and
//! an elevated deploy reach them through.

use crate::prelude::*;
use beet_core::prelude::*;
use std::fmt;
use std::sync::Arc;

/// The erased handle over one provider's deploy credential, landed on the
/// entity of its declaration (`<AwsDeployer/>`, `<CloudflareDeployToken/>`)
/// and collected by [`DeployCredentialQuery`].
///
/// A deploy credential is infrastructure: derived from what the stacks
/// declare, sealed in the repo's secrets document, and kept in line with the
/// declarations by an elevated deploy rather than by anybody's hand. So the
/// two commands a stack has, `deploy` and `deploy --elevated`
/// ([`DeployGate`]), never name a provider: each declared credential answers
/// for itself, and a provider added to an entry adds a credential to the same
/// two commands, never a verb.
#[derive(Clone, Component)]
pub struct DeployCredential {
	provider: Arc<dyn DeployCredentialProvider>,
}

impl fmt::Debug for DeployCredential {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("DeployCredential")
			.field("provider", &self.provider.id())
			.finish()
	}
}

impl DeployCredential {
	/// The key a credential's sealed record carries its grants under, in the
	/// record's [`metadata`](SecretRecord::metadata): one line per grant, what
	/// a plain deploy compares the declarations against.
	pub const GRANTS: &'static str = "grants";

	/// The erased handle over `provider`.
	pub fn new(provider: impl DeployCredentialProvider) -> Self {
		Self {
			provider: Arc::new(provider),
		}
	}

	/// See [`DeployCredentialProvider::id`].
	pub fn id(&self) -> &'static str { self.provider.id() }

	/// See [`DeployCredentialProvider::human_factor`].
	pub fn human_factor(&self) -> &'static str { self.provider.human_factor() }

	/// See [`DeployCredentialProvider::records`].
	pub fn records(&self) -> &'static [&'static str] { self.provider.records() }

	/// See [`DeployCredentialProvider::guards`].
	pub fn guards(&self, declared: &str) -> bool {
		self.provider.guards(declared)
	}

	/// See [`DeployCredentialProvider::protects`].
	pub fn protects(
		&self,
		change: &tofu::PlannedChange,
		stack: &ResolvedStack,
	) -> Option<String> {
		self.provider.protects(change, stack)
	}

	/// See [`DeployCredentialProvider::status`].
	pub async fn status(
		&self,
		caller: &AsyncEntity,
	) -> Result<CredentialStatus> {
		self.provider.status(caller.clone()).await
	}

	/// See [`DeployCredentialProvider::elevate`].
	pub async fn elevate(
		&self,
		caller: &AsyncEntity,
		ask: ElevationAsk,
	) -> Result<CredentialElevation> {
		self.provider.elevate(caller.clone(), ask).await
	}

	/// See [`DeployCredentialProvider::describe`].
	pub async fn describe(&self, caller: &AsyncEntity) -> Result<String> {
		self.provider.describe(caller.clone()).await
	}

	/// Whether this launch holds the sealed credential rather than one passed
	/// for the command: its identity record's value in the environment is the
	/// one the loaded document seals. A credential passed for the command (an
	/// elevated run's, CI's) is the provider's to judge, so only the sealed one
	/// is held to what a stored credential may do.
	pub async fn holds_sealed(&self, caller: &AsyncEntity) -> Result<bool> {
		let Some(record) = self.records().first() else {
			return false.xok();
		};
		let record = *record;
		let sealed = caller
			.with_world(move |world, _| {
				OpenSecrets::find(world, record)
					.ok()
					.map(|secret| secret.value)
			})
			.await?;
		(sealed.is_some()
			&& sealed == env_ext::var(record).ok().map(SmolStr::from))
		.xok()
	}

	/// Every stack this launch declares, rendered at the launch's stage and at
	/// `prod`: what a credential lowers from. A credential is per repo and a
	/// launch runs one stage, so lowering the launch's stage alone would let an
	/// elevated `dev` deploy narrow the credential to what dev declares and
	/// break the next prod deploy. A stack pinned to a stage renders the same
	/// under both, so each app and stage is kept once.
	pub fn render_stages(
		world: &mut World,
	) -> Result<Vec<(ResolvedStack, Deployment, terra::Config)>> {
		let launch = BootstrapConfig::get().stage.clone();
		let prod = SmolStr::new(BootstrapConfig::PROD_STAGE);
		let mut stages = vec![launch];
		if !stages.contains(&prod) {
			stages.push(prod);
		}
		let mut rendered = Vec::new();
		let mut seen = HashSet::<(SmolStr, SmolStr)>::default();
		for stage in stages {
			for scope in RenderScope::render_all_at(world, &stage)? {
				let (stack, deployment, config) = scope.finish()?;
				if seen
					.insert((stack.app_name().clone(), stack.stage().clone()))
				{
					rendered.push((stack, deployment, config));
				}
			}
		}
		rendered.xok()
	}

	/// The grants `secret` was converged with, one per line, empty for a
	/// record that carries none (sealed before credentials recorded them).
	pub fn held_grants(secret: Option<&Secret>) -> Vec<SmolStr> {
		secret
			.and_then(|secret| secret.record.metadata.get(Self::GRANTS))
			.map(|grants| grants.lines().map(SmolStr::from).collect())
			.unwrap_or_default()
	}

	/// The `record` a current credential's sealed `secret` is resealed under,
	/// when what is sealed is not what a converge writes now (a note or roll in
	/// older wording, grants recorded before the declarations changed, another
	/// group), else `None`. Its value is unchanged, so its `modified` is kept.
	pub fn restated(
		secret: &Secret,
		group: &str,
		record: SecretRecord,
	) -> Option<SecretRecord> {
		let record = SecretRecord {
			modified: secret.record.modified,
			..record
		};
		(secret.record != record || secret.group != group).then_some(record)
	}

	/// The sealed record `name` this launch loaded, for a provider's status.
	pub async fn sealed(
		caller: &AsyncEntity,
		name: &'static str,
	) -> Option<Secret> {
		caller
			.with_world(move |world, _| OpenSecrets::find(world, name).ok())
			.await
			.ok()
			.flatten()
	}
}

/// One provider's deploy credential, see [`DeployCredential`]. Implemented by
/// the declaration itself, so the settings it carries are the provider's, and
/// cloned into each future, which owns what it awaits.
pub trait DeployCredentialProvider: 'static + Send + Sync {
	/// Stable provider discriminator, ie `aws`, `cloudflare`.
	fn id(&self) -> &'static str;

	/// What a person supplies to elevate this provider, for a refusal to name
	/// before anybody is asked, ie "the six-digit code from your phone".
	fn human_factor(&self) -> &'static str;

	/// The records the credential is sealed as, its identity first: what an
	/// elevated run strips from the environment of the deploy it runs, so that
	/// deploy loads the values the run just converged rather than inheriting
	/// the ones the launch loaded before them.
	fn records(&self) -> &'static [&'static str];

	/// Whether a change to the `declared` type can be one no stored credential
	/// of this provider may make, so a stack declaring none is never planned
	/// for the check.
	fn guards(&self, declared: &str) -> bool;

	/// Why `change`, planned under `stack`, is one no stored credential of
	/// this provider may make, `None` when it may.
	fn protects(
		&self,
		change: &tofu::PlannedChange,
		stack: &ResolvedStack,
	) -> Option<String>;

	/// What this launch's declarations need of the credential and whether the
	/// sealed one holds it, compared against the grants its record carries.
	/// Reads no account: a plain deploy asks it before every run.
	fn status(
		&self,
		caller: AsyncEntity,
	) -> SendBoxedFuture<Result<CredentialStatus>>;

	/// Ask a person for the human factor, bring the credential in line with
	/// the declarations (sealing its grants), and when `ask.run`, mint what
	/// the elevated deploy runs as.
	fn elevate(
		&self,
		caller: AsyncEntity,
		ask: ElevationAsk,
	) -> SendBoxedFuture<Result<CredentialElevation>>;

	/// What the credential would be converged to, for a dry run: needs no
	/// credential at all.
	fn describe(&self, caller: AsyncEntity) -> SendBoxedFuture<Result<String>>;
}

/// Where one credential stands against the declarations, see
/// [`DeployCredentialProvider::status`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CredentialStatus {
	/// Whether this launch declares anything this provider's credential is
	/// needed for. A credential no stack or route asks for is never elevated.
	pub needed: bool,
	/// Why the sealed credential cannot do what the declarations ask, empty
	/// when it can: a plain deploy refuses on any of them.
	pub stale: Vec<String>,
	/// What is worth saying and blocks nothing, ie a credential wider than the
	/// declarations now need, which the next elevated deploy narrows.
	pub notes: Vec<String>,
}

impl CredentialStatus {
	/// Not needed at all: this launch declares nothing of the provider.
	pub fn not_needed() -> Self { Self::default() }

	/// Compare the grants the declarations need with the ones `record`, the
	/// credential's identity record, was converged with: a grant missing from
	/// the record is stale, one the record holds and the declarations no
	/// longer ask for is a note. `scope` limits the comparison to the held
	/// lines this launch is answerable for (a credential shared by several
	/// entries carries each one's lines), all of them when `None`.
	pub fn compare(
		credential: &str,
		record: &str,
		needed: &[String],
		held: Option<&Secret>,
		scope: Option<&dyn Fn(&str) -> bool>,
	) -> Self {
		let Some(held_secret) = held else {
			return Self {
				needed: true,
				stale: vec![format!(
					"{credential}: no `{record}` is sealed for this repo yet"
				)],
				notes: Vec::new(),
			};
		};
		let held = DeployCredential::held_grants(Some(held_secret));
		if held.is_empty() {
			return Self {
				needed: true,
				stale: vec![format!(
					"{credential}: the sealed `{record}` records no grants, so \
					it was converged before credentials recorded what they \
					hold"
				)],
				notes: Vec::new(),
			};
		}
		let in_scope =
			|line: &str| scope.map(|scope| scope(line)).unwrap_or(true);
		let stale = needed
			.iter()
			.filter(|line| !held.iter().any(|held| held == line.as_str()))
			.map(|line| format!("{credential}: needs `{line}`"))
			.collect::<Vec<_>>();
		let notes = held
			.iter()
			.filter(|line| in_scope(line))
			.filter(|line| !needed.iter().any(|needed| needed == line.as_str()))
			.map(|line| {
				format!(
					"{credential}: holds `{line}`, which nothing declares now; \
					the next elevated deploy drops it"
				)
			})
			.collect();
		Self {
			needed: true,
			stale,
			notes,
		}
	}
}

/// What an elevated deploy asks of one credential, see
/// [`DeployCredentialProvider::elevate`].
#[derive(Debug, Clone)]
pub struct ElevationAsk {
	/// Replace the credential's value even when it is current.
	pub roll: bool,
	/// Mint what the deploy runs as, for a planned change no stored
	/// credential may make. Without it the credential is only converged, and
	/// the deploy runs as the credential it now seals.
	pub run: bool,
	/// The command a person runs in a terminal to elevate, which a run with no
	/// terminal to ask on answers with.
	pub relay: String,
}

/// What elevating one credential produced, see
/// [`DeployCredentialProvider::elevate`].
#[derive(Default)]
pub struct CredentialElevation {
	/// One line per thing converged, for the report.
	pub report: Vec<String>,
	/// What the elevated deploy runs as, by variable: empty when the deploy
	/// runs as the credential the converge sealed.
	pub env: Vec<(SmolStr, SmolStr)>,
	/// What to undo once the deploy exits, however it exits, answering a
	/// report line: the temporary credential's deletion.
	pub cleanup: Option<SendBoxedFuture<Result<String>>>,
}

/// Every [`DeployCredential`] this launch declares.
#[derive(SystemParam)]
pub struct DeployCredentialQuery<'w, 's> {
	credentials: Query<'w, 's, &'static DeployCredential>,
}

impl DeployCredentialQuery<'_, '_> {
	/// Every declared credential, refusing two of one provider: a repo deploys
	/// with one credential per provider, and two declarations would converge
	/// the same token or user to two different lowerings in turn.
	pub fn all(&self) -> Result<Vec<DeployCredential>> {
		let mut seen = HashSet::<&'static str>::default();
		let mut all = Vec::new();
		for credential in self.credentials.iter() {
			if !seen.insert(credential.id()) {
				bevybail!(
					"this launch declares two `{}` deploy credentials, and a \
					repo deploys with one per provider: keep one declaration",
					credential.id()
				);
			}
			all.push(credential.clone());
		}
		all.sort_by_key(|credential| credential.id());
		all.xok()
	}
}

/// Observer: land the [`DeployCredential`] a declaration of `T` declares on
/// its entity, the declaration being the provider.
pub(crate) fn attach_deploy_credential<
	T: 'static + Send + Sync + Clone + Component + DeployCredentialProvider,
>(
	ev: On<Insert, T>,
	declarations: Query<&T>,
	mut commands: Commands,
) {
	if let Ok(declaration) = declarations.get(ev.entity) {
		commands
			.entity(ev.entity)
			.insert(DeployCredential::new(declaration.clone()));
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A held record sealed with `grants`.
	fn held(grants: &str) -> Secret {
		Secret {
			name: "TOKEN".into(),
			group: "default".into(),
			value: "value".into(),
			record: SecretRecord {
				metadata: BTreeMap::from([(
					DeployCredential::GRANTS.into(),
					grants.into(),
				)]),
				..default()
			},
		}
	}

	/// A credential lowers what `prod` declares from a launch of any stage: a
	/// stack taking the launch's stage renders at both, so an elevated `dev`
	/// deploy can never narrow the credential to what dev declares, and a stack
	/// pinned to a stage renders once.
	#[beet_core::test]
	fn renders_the_launch_stage_and_prod() {
		let (deployment, _dir) = Deployment::default_local();
		let mut world = InfraPlugin.into_world();
		world.insert_resource(deployment);
		world.init_resource::<PackageConfig>();
		world.spawn((Stack::new("app"), AwsRegion::new("us-west-2")));
		world.spawn((
			Stack::new("assets").with_stage("shared"),
			AwsRegion::new("us-west-2"),
		));
		let mut rendered = DeployCredential::render_stages(&mut world)
			.unwrap()
			.into_iter()
			.map(|(stack, ..)| {
				format!("{}--{}", stack.app_name(), stack.stage())
			})
			.collect::<Vec<_>>();
		rendered.sort();
		rendered.xpect_eq(vec![
			format!("app--{}", BootstrapConfig::get().stage),
			"app--prod".to_string(),
			"assets--shared".to_string(),
		]);
		// the override is gone once the render is
		world.contains_resource::<RenderStage>().xpect_false();
	}

	/// A grant the declarations need and the record lacks is stale; one the
	/// record holds and nothing asks for is a note, and only within the
	/// launch's scope; no record, or one recording nothing, is stale.
	#[beet_core::test]
	fn compares_grants_with_the_record() {
		let needed = vec!["DNS Write".to_string(), "Cache Purge".to_string()];
		let current = held("Cache Purge\nDNS Write");
		CredentialStatus::compare("cf", "TOKEN", &needed, Some(&current), None)
			.xpect_eq(CredentialStatus {
				needed: true,
				stale: vec![],
				notes: vec![],
			});
		let short = held("DNS Write\nWorkers R2 Storage Write");
		let status = CredentialStatus::compare(
			"cf",
			"TOKEN",
			&needed,
			Some(&short),
			None,
		);
		status
			.stale
			.xpect_eq(vec!["cf: needs `Cache Purge`".to_string()]);
		status.notes.len().xpect_eq(1);
		// another entry's line is not this launch's to drop
		let other_entry = |line: &str| !line.starts_with("Workers");
		CredentialStatus::compare(
			"cf",
			"TOKEN",
			&needed,
			Some(&short),
			Some(&other_entry),
		)
		.notes
		.xpect_empty();
		CredentialStatus::compare("cf", "TOKEN", &needed, None, None).stale[0]
			.as_str()
			.xpect_contains("no `TOKEN` is sealed");
		CredentialStatus::compare(
			"cf",
			"TOKEN",
			&needed,
			Some(&Secret {
				record: default(),
				..held("")
			}),
			None,
		)
		.stale[0]
			.as_str()
			.xpect_contains("records no grants");
	}
}

//! The hierarchical source of truth for cloud resource identity: the authored
//! [`Stack`], the total [`ResolvedStack`] it resolves to, and the traversal
//! every deploy step reaches them through.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The DECLARATION of a resource scope: which app and which stage the
/// resources beneath it belong to, each optional and falling back to this
/// process's own answer, and the tofu-level state settings. Provider-agnostic:
/// where the resources land at a provider is an address spread on the stack
/// or an ancestor ([`AwsRegion`], [`CloudflareAccount`], [`CloudflareZone`]).
///
/// Markup-authorable and registered in every native build, so `<Stack/>` bare
/// works everywhere and `<Stack stage="shared"/>` overrides exactly one field.
/// It is deliberately NOT the thing a name composes from: that is
/// [`ResolvedStack`], which [`resolve`](Self::resolve) produces and which has no
/// optional identity, so a half-answered identity cannot reach a resource name.
#[derive(Debug, Clone, PartialEq, Eq, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
pub struct Stack {
	/// The app identity, else the process [`PackageConfig`]'s. Overriding it is
	/// for a multi-app entry; an app naming itself twice is exactly the drift
	/// this composition exists to prevent.
	#[set_with(unwrap_option, into)]
	app_name: Option<SmolStr>,
	/// The deployment stage namespacing every resource, else this launch's
	/// (`--stage=<x>`, else `BEET_STAGE`, else `dev`). A resource not owned by
	/// any deploy stage declares its own, ie `<Stack stage="shared"/>`.
	#[set_with(unwrap_option, into)]
	stage: Option<SmolStr>,
	/// The environment variable holding the passphrase this stack's tofu
	/// state and plans are encrypted with, see [`StateEncryption`]:
	/// [`DEFAULT_STATE_PASSPHRASE`](Self::DEFAULT_STATE_PASSPHRASE) unless
	/// declared, a record of the entry's secrets document shared by every
	/// stack the repo declares, so a new stack's state is encrypted by
	/// forgetting rather than plaintext by forgetting, and a missing variable
	/// is an error at `tofu init`, never a silent downgrade. A throwaway
	/// stack opts out with `state_passphrase={None}`.
	#[set_with(into)]
	state_passphrase: Option<SmolStr>,
	/// The one-deploy migration switch: the apply that turns encryption on
	/// reads the existing plaintext state through the `unencrypted` fallback
	/// and writes it back encrypted; off again for the apply after, which
	/// then plans no change.
	state_migrate: bool,
	/// Additional parameters, some of which may be required by a config
	/// generator.
	#[reflect(ignore)]
	params: MultiMap<SmolStr, SmolStr>,
}

impl Default for Stack {
	fn default() -> Self {
		Self {
			app_name: None,
			stage: None,
			state_passphrase: Some(Self::DEFAULT_STATE_PASSPHRASE.into()),
			state_migrate: false,
			params: MultiMap::default(),
		}
	}
}

impl Stack {
	/// The environment variable every stack's state passphrase is read from
	/// unless declared, one record per repo (`secrets/set TF_STATE_PASSPHRASE
	/// --generate`).
	pub const DEFAULT_STATE_PASSPHRASE: &'static str = "TF_STATE_PASSPHRASE";

	/// A stack naming `app_name` explicitly, the code counterpart of
	/// `<Stack app_name=".."/>`.
	pub fn new(app_name: impl Into<SmolStr>) -> Self {
		Self::default().with_app_name(app_name)
	}

	/// Fill every unset identity field from this process's answer, the ONE
	/// place a stack consults [`PackageConfig`] or [`BootstrapConfig`]. The
	/// provider addresses are not this declaration's to answer: outside a
	/// world they stay unset, and [`StackQuery::resolve`] reads them by
	/// ancestry.
	///
	/// Total by construction: `package` is required and its
	/// [`app_name`](PackageConfig::app_name) always set, so there is no
	/// half-resolved outcome and nothing downstream re-asks the question.
	pub fn resolve(&self, package: &PackageConfig) -> ResolvedStack {
		ResolvedStack {
			app_name: self
				.app_name
				.clone()
				.unwrap_or_else(|| package.app_name().into()),
			stage: self
				.stage
				.clone()
				.unwrap_or_else(|| BootstrapConfig::get().stage.clone()),
			region: None,
			cloudflare_account: None,
			cloudflare_zone: None,
			state_encryption: self
				.state_passphrase
				.clone()
				.map(|env_var| {
					StateEncryption::passphrase(env_var)
						.with_migrate(self.state_migrate)
				})
				.unwrap_or_default(),
			params: self.params.clone(),
		}
	}
}

/// A [`Stack`] with its identity answered (the app and stage that turn a
/// resource *label* into a resource *name*, ie the `beet-site` + `prod`
/// behind `beet-site--prod--analytics`) and the provider addresses the
/// asking entity resolves by ancestry, each answered on demand by name.
///
/// A declaration carries only its label, and BOTH meanings of that declaration
/// compose the name here: the deploy that creates the resource and the runtime
/// that reads or writes it. One composition, so the two cannot drift, and it
/// takes a RESOLVED stack so a name can never compose from a scope that was
/// never resolved.
///
/// [`Stack::resolve`] builds one with no addresses and [`StackQuery::resolve`]
/// fills them in, so every store is handed the region it was declared under
/// rather than reaching for one itself.
#[derive(Debug, Clone, PartialEq, Eq, Get, SetWith)]
pub struct ResolvedStack {
	app_name: SmolStr,
	stage: SmolStr,
	/// The nearest [`AwsRegion`], see [`region`](Self::region).
	#[get(skip)]
	#[set_with(unwrap_option, into)]
	region: Option<SmolStr>,
	/// The nearest [`CloudflareAccount`], see
	/// [`cloudflare_account`](Self::cloudflare_account).
	#[get(skip)]
	#[set_with(unwrap_option)]
	cloudflare_account: Option<CloudflareAccount>,
	/// The nearest [`CloudflareZone`], see
	/// [`cloudflare_zone`](Self::cloudflare_zone).
	#[get(skip)]
	#[set_with(unwrap_option)]
	cloudflare_zone: Option<CloudflareZone>,
	/// How the stack's state is encrypted, [`StateEncryption::None`] unless
	/// the declaration names a passphrase; rendered onto this launch's
	/// [`Deployment`] when the stack's config is seeded.
	state_encryption: StateEncryption,
	params: MultiMap<SmolStr, SmolStr>,
}

impl ResolvedStack {
	/// The aws region every resource in this stack deploys into, or an error
	/// naming the stack and the spread that declares one: a region that could
	/// fall back would silently plan a full replacement.
	pub fn region(&self) -> Result<&SmolStr> {
		self.region.as_ref().ok_or_else(|| {
			bevyhow!(
				"stack `{}--{}` declares no aws region: declare \
				`{{AwsRegion(\"..\")}}` on the stack or an ancestor",
				self.app_name,
				self.stage
			)
		})
	}

	/// The Cloudflare account this stack's Cloudflare resources belong to, or
	/// an error naming the stack and the spread that declares one.
	pub fn cloudflare_account(&self) -> Result<&CloudflareAccount> {
		self.cloudflare_account.as_ref().ok_or_else(|| {
			bevyhow!(
				"stack `{}--{}` declares no cloudflare account: declare \
				`{{CloudflareAccount{{id:\"..\"}}}}` on the stack or an ancestor",
				self.app_name,
				self.stage
			)
		})
	}

	/// The Cloudflare zone this stack publishes into, or an error naming the
	/// stack and the spread that declares one. A record's zone goes through
	/// [`cloudflare_zone_holding`](Self::cloudflare_zone_holding); this is
	/// for the zone-level verbs (a cache purge, the zone settings, an audit).
	pub fn cloudflare_zone(&self) -> Result<&CloudflareZone> {
		self.cloudflare_zone.as_ref().ok_or_else(|| {
			bevyhow!(
				"stack `{}--{}` declares no cloudflare zone: declare \
				`{{CloudflareZone{{domain:\"..\", id:\"..\"}}}}` on the stack \
				or an ancestor",
				self.app_name,
				self.stage
			)
		})
	}

	/// The Cloudflare zone `record_name` publishes into: the declared zone,
	/// which must hold the name (its apex or a name under it), else an error
	/// naming both, the cheap guard that catches a wrong spread.
	pub fn cloudflare_zone_holding(
		&self,
		record_name: &str,
	) -> Result<&CloudflareZone> {
		let zone = self
			.cloudflare_zone()
			.map_err(|err| bevyhow!("publishing `{record_name}`: {err}"))?;
		if !zone.holds(record_name) {
			bevybail!(
				"stack `{}--{}` publishes `{record_name}` but its zone is \
				`{}`: a record outside the declared zone needs its own \
				`{{CloudflareZone{{..}}}}` spread",
				self.app_name,
				self.stage,
				zone.domain
			);
		}
		Ok(zone)
	}

	/// Whether this stack deploys the [production
	/// stage](BootstrapConfig::PROD_STAGE), which often receives special
	/// treatment like bucket locking and no subdomain.
	pub fn is_production(&self) -> bool {
		self.stage == BootstrapConfig::PROD_STAGE
	}

	/// The identifier a resource label composes to in this stack, the single
	/// definition of the `app--stage--label` convention.
	pub fn resource_ident(&self, label: impl Into<SmolStr>) -> terra::Ident {
		terra::Ident::new(self.app_name.clone(), self.stage.clone(), label)
	}

	/// The provider-facing resource name, ie `beet-site--prod--analytics`.
	pub fn resource_name(&self, label: impl Into<SmolStr>) -> String {
		self.resource_ident(label).primary_identifier().to_string()
	}

	/// A resolved stack plus the launch that deploys it locally: a local state
	/// backend and a temporary work directory removed on drop.
	#[cfg(test)]
	pub(crate) fn default_local()
	-> (Self, Deployment, crate::types::TestWorkDir) {
		let (deployment, dir) = Deployment::default_local();
		(Self::test_local(), deployment, dir)
	}

	/// The default test stack resolved with the region the tests pin.
	#[cfg(test)]
	pub(crate) fn test_local() -> Self {
		Stack::new("beet_infra")
			.resolve(&PackageConfig::default())
			.with_region(Stack::TEST_REGION)
	}
}

#[cfg(test)]
impl Stack {
	/// The region every rendered test value is pinned to.
	pub(crate) const TEST_REGION: &'static str = "us-west-2";

	/// The default test stack with its region declared, the root a block
	/// test spawns under.
	pub(crate) fn test_local() -> (Self, AwsRegion) {
		(Stack::new("beet_infra"), AwsRegion::new(Self::TEST_REGION))
	}

	/// The zone id every zoned test stack declares.
	pub(crate) const TEST_ZONE_ID: &'static str = "zone123";

	/// [`test_local`](Self::test_local) with a Cloudflare zone for `domain`,
	/// for a block publishing records into it.
	pub(crate) fn test_zoned(
		domain: &str,
	) -> (Self, AwsRegion, CloudflareZone) {
		let (stack, region) = Self::test_local();
		(
			stack,
			region,
			CloudflareZone::new(domain, Self::TEST_ZONE_ID),
		)
	}
}

/// Resolves the [`Stack`] an entity belongs to and the provider addresses
/// above it, and the deploy traversal that starts from it. Rendering the
/// stack's config is not here: every caller, including tests and wasm
/// consumers, holds a `World` and renders through [`RenderScope::render`]
/// (the schedule is target-agnostic).
#[derive(SystemParam)]
pub struct StackQuery<'w, 's> {
	stacks: AncestorQuery<'w, 's, (Entity, &'static Stack)>,
	/// The addresses, each the nearest ancestor-or-self of the asking entity,
	/// so a shared one sits on the root and a block needing another carries
	/// its own spread.
	regions: AncestorQuery<'w, 's, &'static AwsRegion>,
	accounts: AncestorQuery<'w, 's, &'static CloudflareAccount>,
	zones: AncestorQuery<'w, 's, &'static CloudflareZone>,
	children: Query<'w, 's, &'static Children>,
	stores: Query<'w, 's, &'static BlobStore>,
	/// The process app identity, which [`BootstrapPlugin`] inserts at build time
	/// and [`InfraPlugin`] therefore guarantees, so resolution is total.
	package: Res<'w, PackageConfig>,
	deployment: Option<Res<'w, Deployment>>,
	/// The secret store a declaration landed, see
	/// [`secret_store`](Self::secret_store).
	#[cfg(feature = "vault")]
	secret_stores: Query<'w, 's, &'static SecretStore>,
}

impl<'w, 's> StackQuery<'w, 's> {
	/// The [`ResolvedStack`] `entity` composes its resource names against (the
	/// nearest ancestor's, else the process default) carrying the provider
	/// addresses nearest `entity` itself. A declaration outside every stack is
	/// not an error, it simply belongs to no deploy's config and resolves the
	/// names the process itself would.
	pub fn resolve(&self, entity: Entity) -> ResolvedStack {
		let mut stack = self
			.stack(entity)
			.cloned()
			.unwrap_or_default()
			.resolve(&self.package);
		stack.region =
			self.regions.get(entity).ok().map(|region| region.0.clone());
		stack.cloudflare_account = self.accounts.get(entity).ok().cloned();
		stack.cloudflare_zone = self.zones.get(entity).ok().cloned();
		stack
	}

	/// The nearest ancestor [`Stack`] as authored, if any. A declaration made
	/// purely for its runtime meaning has none.
	pub fn stack(&self, entity: Entity) -> Option<&Stack> {
		self.stacks.get(entity).ok().map(|(_, stack)| stack)
	}

	/// This launch's [`Deployment`], which [`InfraPlugin`] inits, else the
	/// derived default for a world that has no infra plugin.
	pub fn deployment(&self) -> Deployment {
		self.deployment
			.as_deref()
			.cloned()
			.unwrap_or_else(Deployment::default)
	}

	/// This launch's deploy id, the version every artifact it publishes is
	/// keyed by, read from the [`Deployment`] resource. A world without
	/// [`InfraPlugin`] has no resource, so unless the launch names one
	/// (`--deploy-id`) each call mints a fresh id: a caller reads it once.
	pub fn deploy_id(&self) -> Uuid {
		self.deployment
			.as_deref()
			.map(|deployment| *deployment.deploy_id())
			.unwrap_or_else(|| *Deployment::default().deploy_id())
	}

	/// The entity carrying the nearest ancestor [`Stack`], and that stack
	/// resolved as the root sees it: the root every block, artifact and verb
	/// under one deploy resolves against.
	pub fn root(&self, entity: Entity) -> Result<(Entity, ResolvedStack)> {
		let (root, _) = self.stacks.get(entity)?;
		Ok((root, self.resolve(root)))
	}

	/// Every entity declared under `entity`'s stack: its root's inclusive
	/// descendants, full stop. The one traversal a deploy step uses to find what
	/// was declared alongside it.
	///
	/// A resource belongs to the stack it is authored under and to no other, so
	/// a declaration outside every `<Stack>` belongs to no deploy's config: it
	/// still resolves the process default for its runtime meaning, but nothing
	/// provisions it. That is the unrepresentable-by-construction answer to
	/// "which stack owns this?", replacing a sweep that inferred it from the
	/// process stage and could quietly provision a resource into the wrong one.
	pub fn declared(&self, entity: Entity) -> Result<Vec<Entity>> {
		let (root, _) = self.root(entity)?;
		self.children
			.iter_descendants_inclusive(root)
			.collect::<Vec<_>>()
			.xok()
	}

	/// Get the [`BlobStore`] component from this entity.
	pub fn store(&self, entity: Entity) -> Result<&BlobStore> {
		self.stores.get(entity)?.xok()
	}

	/// The [`SecretStore`] `entity`'s stack keeps its secrets in, scoped to
	/// that stack: the one declared on or under its `<Stack>`
	/// (`<SsmSecrets/>`, `<DocumentSecrets path=".."/>`), else the implicit
	/// `<SsmSecrets/>` every stack carries ([`SsmSecrets::store`]). Two
	/// declarations under one stack is an error naming both, never a guess.
	#[cfg(feature = "vault")]
	pub fn secret_store(&self, entity: Entity) -> Result<SecretStore> {
		let root = self
			.stacks
			.get(entity)
			.map(|(root, _)| root)
			.unwrap_or(entity);
		let stack = self.resolve(entity);
		let mut declared = self
			.children
			.iter_descendants_inclusive(root)
			.filter_map(|child| self.secret_stores.get(child).ok());
		match (declared.next(), declared.next()) {
			(Some(store), None) => store.clone().xok(),
			(Some(first), Some(second)) => bevybail!(
				"stack `{}--{}` declares two secret stores ({} and {}): a stack \
				keeps its secrets in exactly one",
				stack.app_name(),
				stack.stage(),
				first.describe(),
				second.describe()
			),
			(None, _) => SsmSecrets::store(stack),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A stack resolved against an app that names itself.
	fn resolved(stack: Stack) -> ResolvedStack {
		stack.resolve(&PackageConfig {
			app_name: "beet-site".into(),
			..default()
		})
	}

	/// The composition is the one the live stacks already resolve; renaming a
	/// deployed resource is a production incident, so these strings are pinned.
	#[beet_core::test]
	fn composes_the_live_names() {
		let prod = resolved(Stack::default().with_stage("prod"));
		prod.resource_name("analytics")
			.xpect_eq("beet-site--prod--analytics");
		prod.resource_name("app").xpect_eq("beet-site--prod--app");
		resolved(Stack::default().with_stage("dev"))
			.resource_name("repo")
			.xpect_eq("beet-site--dev--repo");
		resolved(Stack::default().with_stage("shared"))
			.resource_name("assets")
			.xpect_eq("beet-site--shared--assets");
		resolved(Stack::new("beet").with_stage("shared"))
			.resource_name("assets")
			.xpect_eq("beet--shared--assets");
	}

	/// Every unset field falls back to this process's answer: the app name to
	/// the [`PackageConfig`], and the stage to the launch (the beet test runner
	/// passes no `--stage`/`BEET_STAGE`, so it is `dev` and not production).
	#[beet_core::test]
	fn resolves_the_process_defaults() {
		let stack = resolved(Stack::default());
		stack.app_name().as_str().xpect_eq("beet-site");
		stack.stage().as_str().xpect_eq("dev");
		stack.is_production().xpect_false();
	}

	/// An app that declared no name still composes a name rather than an empty
	/// segment, and both meanings of a declaration compose the SAME one, which
	/// is what makes a generic fallback safe (the live incident it guards was
	/// two INDEPENDENT derivations, not the fallback itself).
	#[beet_core::test]
	fn an_unnamed_app_resolves_the_generic_name() {
		Stack::default()
			.resolve(&PackageConfig::default())
			.resource_name("analytics")
			.xpect_eq("beet-app--dev--analytics");
	}

	/// The `prod` stage (what `--stage=prod` resolves to) marks production,
	/// flipping the stage-aware paths (eg the beet-site apex dns).
	#[beet_core::test]
	fn prod_stage_is_production() {
		resolved(Stack::default().with_stage("prod"))
			.is_production()
			.xpect_true();
	}

	/// An authored field wins over the process answer, and resolution never
	/// clobbers it.
	#[beet_core::test]
	fn authored_fields_win() {
		let stack = resolved(
			Stack::new("other-app")
				.with_stage("shared")
				.with_state_passphrase(None),
		);
		stack.app_name().as_str().xpect_eq("other-app");
		stack.stage().as_str().xpect_eq("shared");
		stack.state_encryption().xpect_eq(StateEncryption::None);
	}

	/// A stack outside a world has no region, and asking for one names the
	/// stack and the spread that declares it rather than falling back.
	#[beet_core::test]
	fn an_undeclared_region_fails_by_name() {
		resolved(Stack::default().with_stage("prod"))
			.region()
			.unwrap_err()
			.to_string()
			.xpect_contains("beet-site--prod")
			.xpect_contains("AwsRegion");
		resolved(Stack::default())
			.with_region("eu-west-1")
			.region()
			.unwrap()
			.as_str()
			.xpect_eq("eu-west-1");
	}

	/// The state is encrypted by default, under the repo's one passphrase
	/// variable, its migration switch carried along; a declared variable
	/// replaces it and an explicit `None` opts out.
	#[beet_core::test]
	fn the_state_is_encrypted_by_default() {
		resolved(Stack::default())
			.state_encryption()
			.xpect_eq(StateEncryption::passphrase("TF_STATE_PASSPHRASE"));
		resolved(Stack {
			state_migrate: true,
			..default()
		})
		.state_encryption()
		.xpect_eq(
			StateEncryption::passphrase("TF_STATE_PASSPHRASE")
				.with_migrate(true),
		);
		resolved(Stack::default().with_state_passphrase(SmolStr::new("OTHER")))
			.state_encryption()
			.xpect_eq(StateEncryption::passphrase("OTHER"));
		resolved(Stack::default().with_state_passphrase(None))
			.state_encryption()
			.xpect_eq(StateEncryption::None);
	}

	/// The markup opt-out is the explicit `None` literal, which the coercion
	/// passes through onto the `Option` field; a bare declaration keeps the
	/// default.
	#[beet_core::test]
	fn the_none_literal_opts_out() {
		let mut world = InfraPlugin.into_world();
		BsxTemplate::parse_entry(
			&world,
			r#"<Fragment>
				<Stack app_name="throwaway" state_passphrase={None}/>
				<Stack app_name="kept"/>
				<Stack app_name="other" state_passphrase="OTHER"/>
			</Fragment>"#,
		)
		.unwrap()
		.spawn(&mut world)
		.unwrap();
		world.flush();
		world
			.query::<&Stack>()
			.iter(&world)
			.map(|stack| stack.state_passphrase.clone())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				None,
				Some(SmolStr::new("TF_STATE_PASSPHRASE")),
				Some(SmolStr::new("OTHER")),
			]);
	}

	/// The storage layer the beet site's stage stack renders: the app and
	/// analytics buckets, both named through the one composition and in the
	/// stack's declared region.
	///
	/// These are live resources. A moved name or region replaces them, so the
	/// values are pinned rather than the shape.
	#[beet_core::test]
	fn the_storage_layer_renders_the_live_names() {
		let (scope, _dir) = RenderScope::test_render_stack(
			(
				Stack::new("beet-site").with_stage("prod"),
				AwsRegion::new("us-west-2"),
			),
			|parent| {
				parent.spawn(
					S3BucketBlock::new("app").with_deploy_versioned(false),
				);
				parent.spawn(
					S3BucketBlock::new("analytics").with_runtime_write(true),
				);
			},
		);
		let (_stack, _deployment, config) = scope.finish().unwrap();
		config
			.to_json_string()
			.unwrap()
			.as_str()
			.xpect_contains("\"bucket\":\"beet-site--prod--app\"")
			.xpect_contains("\"bucket\":\"beet-site--prod--analytics\"")
			.xpect_contains("\"region\":\"us-west-2\"");
		// both converge in the layer applied before anything that reads them
		config.layer_targets("storage").unwrap().len().xpect_eq(2);
	}

	/// Two stacks sharing one launch compose distinct state paths, so a `shared`
	/// deploy can never overwrite the stage deploy's state. The keys are the ones
	/// the live backends already hold (the suffix kebab-cases with every other
	/// segment), so they are pinned too.
	#[beet_core::test]
	fn stacks_share_a_launch_and_split_their_state() {
		let (deployment, _dir) = Deployment::default_local();
		let stage = resolved(Stack::default().with_stage("dev"));
		let shared = resolved(Stack::default().with_stage("shared"));
		deployment
			.backend_path(&stage)
			.to_string()
			.xpect_eq("beet-site--dev--tofu-tfstate");
		deployment
			.backend_path(&shared)
			.to_string()
			.xpect_eq("beet-site--shared--tofu-tfstate");
	}
}

//! `<AtprotoHandleBlock/>`: the DNS records that make a domain's names atproto
//! handles.
//!
//! A custom-domain handle is one DNS TXT record. `pete.beet.org` is a handle
//! because `_atproto.pete.beet.org` holds `did=did:plc:...`, and that record is
//! the entire infrastructure: the account it names lives on somebody's PDS,
//! keeps its did across every rename, and is declared once as an
//! `<AtprotoAccount/>`. So the block is a zone declaration naming accounts,
//! not an account one, and the probe is the only step that talks to the
//! network at all.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Every custom-domain atproto handle published under one domain, ie the
/// `beet.org` behind `pete.beet.org`, each read off an `<AtprotoAccount/>`:
///
/// ```html
/// <AtprotoAccount bx:ref="beet_org" handle="beet.org" did="did:plc:.." display_name="Beet"/>
/// <AtprotoAccount bx:ref="pete" handle="pete.beet.org" did="did:plc:.." display_name="Pete"/>
/// <AtprotoHandleBlock domain="beet.org" dns_stage="prod" accounts={[$beet_org, $pete]}/>
/// ```
///
/// One TXT record per account, `_atproto.<name>.<domain>` holding
/// `did=did:plc:...`, or `_atproto.<domain>` for the apex handle, which takes
/// no name because it IS the domain. Each account's handle must be the domain
/// or one label under it.
///
/// A zone is owned by one entry (see `ZoneAudit`), so a handle domain that is
/// also a mail domain declares this tag inside THAT stack rather than in a
/// second one: the block is its own tag precisely so it can be authored
/// wherever the zone's owner is.
#[derive(
	Debug, Default, Clone, Get, SetWith, Component, Reflect, MapEntities,
)]
#[reflect(Component, Default, MapEntities)]
#[component(immutable, on_insert = ErasedBlock::on_insert::<Self>,
	on_remove = ErasedBlock::on_remove
)]
pub struct AtprotoHandleBlock {
	/// The domain handles hang off, ie the `beet.org` in
	/// `_atproto.pete.beet.org`.
	domain: SmolStr,
	/// The `<AtprotoAccount/>` declarations whose handles this domain
	/// publishes.
	#[entities]
	#[set_with(skip)]
	accounts: Vec<Entity>,
	/// The zone the records are published into. Without one a block declaring
	/// handles cannot publish them at all, which is an error rather than a
	/// silent no-op: a green deploy that published no record is the one failure
	/// a handle stack cannot see.
	#[get(skip)]
	#[set_with(unwrap_option)]
	dns: Option<DnsProvider>,
	/// The stage that OWNS these names, ie the one whose deploy may publish
	/// them.
	///
	/// A stack's resources are named `<app>--<stage>--<label>`, but a handle
	/// record is not: `_atproto.pete.beet.org` is the real name of a real
	/// identity, and a second stage deploying the same declaration would
	/// publish a SECOND TXT record at it. Resolvers do not merge those, so the
	/// handle stops resolving deterministically and the account's name breaks
	/// for everybody.
	///
	/// So a stage that is not this one publishes nothing. Empty means no guard,
	/// which is right for a domain no other stage deploys.
	#[set_with(unwrap_option, into)]
	dns_stage: Option<SmolStr>,
}

impl AtprotoHandleBlock {
	/// The resource-label prefix every handle record composes under, ie the
	/// `atproto-pete` behind `beet-org--atproto-pete`, or the `atproto-apex`
	/// an apex handle takes in place of the name it has none of.
	pub const RECORD_LABEL: &'static str = "atproto";

	/// A handle domain publishing no account yet.
	pub fn new(domain: impl Into<SmolStr>) -> Self {
		Self {
			domain: domain.into(),
			..default()
		}
	}

	/// Publish the handle of the account declared on `account`.
	pub fn with_account(mut self, account: Entity) -> Self {
		self.accounts.push(account);
		self
	}

	/// The provider this domain's records are published through: the
	/// declared [`dns`](Self::with_dns) provider, else Cloudflare records in
	/// the stack's `CloudflareZone`, resolved at render from the block's
	/// ancestry.
	pub fn resolved_dns(&self) -> DnsProvider {
		self.dns
			.clone()
			.unwrap_or_else(|| DnsProvider::cloudflare(self.domain.clone()))
	}

	/// Whether this stage may publish these names, ie whether it is the
	/// [`dns_stage`](Self::dns_stage) (or none was declared).
	pub fn owns_names(&self, stack: &ResolvedStack) -> bool {
		self.dns_stage
			.as_ref()
			.is_none_or(|owner| owner == stack.stage())
	}

	/// The domain with its dots as hyphens, ie `beet-org`, where a name must be
	/// unique per domain but cannot contain a dot, ie the terraform labels.
	pub fn slug(&self) -> String { self.domain.replace('.', "-") }

	/// The handle each declared account publishes, rejecting a declaration
	/// that cannot: a domain that is not a legal name, an entity that is not
	/// an account, a handle outside the domain or more than one label under
	/// it, or one name claimed twice.
	///
	/// Checked at render rather than at apply, since every one of these is a
	/// typo and the cheapest place to catch a typo is before any record exists.
	pub fn handles(
		&self,
		accounts: &Query<&AtprotoAccount>,
	) -> Result<Vec<AtprotoHandle>> {
		DnsProvider::validate_label(&self.slug(), "handle domain")?;
		let mut seen = HashSet::<Option<SmolStr>>::default();
		self.accounts
			.iter()
			.map(|entity| {
				let account = accounts.get(*entity).map_err(|_| {
					bevyhow!(
						"'{}' publishes entity {entity}, which is not an \
						 `<AtprotoAccount/>`",
						self.domain
					)
				})?;
				let handle = AtprotoHandle::new(&self.domain, account)?;
				if !seen.insert(handle.name().clone()) {
					bevybail!(
						"'{}' is declared twice on '{}': two records at one \
						 name is a handle that resolves to neither",
						handle.handle(&self.domain),
						self.domain
					);
				}
				handle.xok()
			})
			.collect()
	}

	/// A terraform label for this domain's `suffix` resource, distinct from
	/// every other domain's in the same stack.
	fn label(&self, suffix: &str) -> String {
		format!("{}--{suffix}", self.slug())
	}

	/// The [`DeployRender`] render system, registered by [`InfraPlugin`]:
	/// resolve each block's accounts, then emit one TXT record per handle.
	pub(crate) fn render(
		mut scopes: AncestorQuery<&mut RenderScope>,
		stacks: StackQuery,
		blocks: Query<(Entity, &AtprotoHandleBlock)>,
		accounts: Query<&AtprotoAccount>,
	) {
		for (entity, block) in blocks.iter() {
			let Ok(mut scope) = scopes.get_mut(entity) else {
				continue;
			};
			let stack = stacks.resolve(entity);
			let (_deployment, config) = scope.ctx();
			if let Err(err) = block
				.handles(&accounts)
				.and_then(|handles| block.emit(&stack, config, &handles))
			{
				scope.error(bevyhow!(
					"AtprotoHandleBlock '{}': {err}",
					block.domain
				));
			}
		}
	}

	/// One TXT record per handle, or none at all for a stage that does not own
	/// these names.
	fn emit(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		handles: &[AtprotoHandle],
	) -> Result {
		if handles.is_empty() {
			return Ok(());
		}
		if !self.owns_names(stack) {
			warn!(
				"stage '{}' does not own '{}', so its handle records are not published",
				stack.stage(),
				self.domain
			);
			return Ok(());
		}
		let dns = self.resolved_dns();
		for handle in handles {
			dns.emit_txt(
				stack,
				config,
				&self.label(&format!(
					"{}-{}",
					Self::RECORD_LABEL,
					handle.label()
				)),
				&handle.record_name(&self.domain),
				&handle.record_value(),
			)?;
		}
		Ok(())
	}
}

impl Block for AtprotoHandleBlock {
	/// The name this block is known by is its domain.
	fn label(&self) -> &SmolStr { &self.domain }
}

/// One custom-domain handle resolved off its account: the label it takes
/// under its block's domain, and the did it resolves to.
///
/// A handle carries a name only if it takes one. Without one the handle IS
/// the domain, ie `beet.org` rather than `pete.beet.org`, which is the one
/// handle a company account usually wants.
#[derive(Debug, Clone, PartialEq, Eq, Get)]
pub struct AtprotoHandle {
	/// The label under the handle domain, ie the `pete` in `pete.beet.org`.
	/// Absent is the APEX: the handle is the bare domain.
	name: Option<SmolStr>,
	/// The account this handle resolves to.
	did: Did,
}

impl AtprotoHandle {
	/// The label an apex handle's record composes under, standing in for the
	/// name it does not have: `beet-org--atproto-apex` rather than a
	/// trailing-hyphen `atproto-`.
	///
	/// A handle named literally `apex` beside an apex handle composes this same
	/// label, which the config's duplicate-resource check rejects at emit: two
	/// records at one label is one of them silently replacing the other in
	/// state, so it fails loudly instead.
	pub const APEX_LABEL: &'static str = "apex";

	/// The handle `account` takes under `domain`: the domain itself, or one
	/// legal, unreserved label under it.
	pub fn new(domain: &str, account: &AtprotoAccount) -> Result<Self> {
		let handle = account.handle().to_ascii_lowercase();
		let name = match handle.strip_suffix(domain) {
			Some("") => None,
			Some(prefix) if prefix.ends_with('.') => {
				Some(SmolStr::new(&prefix[..prefix.len() - 1]))
			}
			_ => bevybail!(
				"the handle '{handle}' is not under '{domain}': a handle \
				 block publishes only its own domain's names"
			),
		};
		if let Some(name) = &name {
			DnsProvider::validate_label(name, "handle name")?;
			if DnsProvider::RESERVED_HOSTNAMES.contains(&name.as_str()) {
				bevybail!(
					"handle name '{name}' is a reserved hostname: handles share \
					 the zone with infrastructure names"
				);
			}
		}
		Self {
			name,
			did: account.did().clone(),
		}
		.xok()
	}

	/// The handle itself, ie `pete.beet.org`, or the bare `beet.org` for the
	/// apex.
	pub fn handle(&self, domain: &str) -> String {
		match &self.name {
			Some(name) => format!("{name}.{domain}"),
			None => domain.to_string(),
		}
	}

	/// The record this handle is published at, ie `_atproto.pete.beet.org`,
	/// or `_atproto.beet.org` for the apex.
	///
	/// Always nested under `_atproto.`, which is a name under the apex even for
	/// the apex handle, so a zone's apex-safety rules never have to make an
	/// exception for it: the apex HANDLE publishes no record AT the apex.
	pub fn record_name(&self, domain: &str) -> String {
		format!("_atproto.{}", self.handle(domain))
	}

	/// The label fragment this handle's record composes under, ie the `pete` in
	/// `beet-org--atproto-pete`, or [`APEX_LABEL`](Self::APEX_LABEL).
	pub fn label(&self) -> &str {
		self.name.as_deref().unwrap_or(Self::APEX_LABEL)
	}

	/// The record value, ie `did=did:plc:...`. The `did=` is the spec's, not
	/// decoration: a bare did in this record resolves nothing.
	pub fn record_value(&self) -> String { format!("did={}", self.did) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use serde_json::Value;

	const ALICE: &str = "did:plc:aaaaaaaaaaaaaaaaaaaaaaaa";
	const BOB: &str = "did:plc:bbbbbbbbbbbbbbbbbbbbbbbb";
	const COMPANY: &str = "did:plc:cccccccccccccccccccccccc";

	fn account(handle: &str, did: &str) -> AtprotoAccount {
		AtprotoAccount::new(handle, Did::parse(did).unwrap(), handle)
	}

	/// Render `example.com` publishing `accounts` under `stack`, the block
	/// configured by `block`: the scope with its errors still inside, and the
	/// work dir it renders against.
	fn render(
		stack: Stack,
		accounts: &[(&str, &str)],
		block: impl FnOnce(AtprotoHandleBlock) -> AtprotoHandleBlock,
	) -> (RenderScope, crate::types::TestWorkDir) {
		let accounts = accounts.to_vec();
		RenderScope::test_render_stack(
			(
				stack,
				AwsRegion::new("us-west-2"),
				CloudflareZone::new("example.com", "zone123"),
			),
			|parent| {
				let declared = accounts
					.iter()
					.map(|(handle, did)| {
						parent.spawn(account(handle, did)).id()
					})
					.collect::<Vec<_>>();
				parent.spawn(block(declared.into_iter().fold(
					AtprotoHandleBlock::new("example.com"),
					AtprotoHandleBlock::with_account,
				)));
			},
		)
	}

	/// The `(label, name, content)` of every record rendered, sorted by name.
	fn rendered(
		stack: Stack,
		accounts: &[(&str, &str)],
		block: impl FnOnce(AtprotoHandleBlock) -> AtprotoHandleBlock,
	) -> Vec<(String, String, String)> {
		let (scope, _dir) = render(stack, accounts, block);
		let json = scope.finish().unwrap().2.to_json().into_json();
		let Some(Value::Object(records)) = json
			.get("resource")
			.and_then(|resource| resource.get("cloudflare_dns_record"))
		else {
			return Vec::new();
		};
		let mut records = records
			.iter()
			.map(|(label, record)| {
				let field = |key: &str| {
					record
						.get(key)
						.and_then(Value::as_str)
						.unwrap_or_default()
						.to_string()
				};
				(label.clone(), field("name"), field("content"))
			})
			.collect::<Vec<_>>();
		records.sort_by(|a, b| a.1.cmp(&b.1));
		records
	}

	/// The `(name, content)` of every record rendered.
	fn records(
		stack: Stack,
		accounts: &[(&str, &str)],
	) -> Vec<(String, String)> {
		rendered(stack, accounts, |block| block)
			.into_iter()
			.map(|(_, name, content)| (name, content))
			.collect()
	}

	/// The first error a render collected.
	fn render_err(accounts: &[(&str, &str)]) -> String {
		let (scope, _dir) =
			render(Stack::new("atproto"), accounts, |block| block);
		scope.finish().unwrap_err().to_string()
	}

	/// A handle IS its record, so the pair is pinned in full: a wrong name is a
	/// handle that never resolves, and a value missing the `did=` prefix is a
	/// record that exists and means nothing.
	#[beet_core::test]
	fn a_handle_is_one_txt_record() {
		records(Stack::new("atproto"), &[
			("alice.example.com", ALICE),
			("bob.example.com", BOB),
		])
		.xpect_eq(vec![
			("_atproto.alice.example.com".into(), format!("did={ALICE}")),
			("_atproto.bob.example.com".into(), format!("did={BOB}")),
		]);
	}

	/// The apex handle is the domain itself, so the ONE thing that changes is
	/// the name: still `_atproto.`-prefixed, but with no label between the
	/// prefix and the domain. Beside a subdomain it is a distinct record at a
	/// distinct terraform label, the apex's written out since it has no name
	/// to compose one from.
	#[beet_core::test]
	fn the_apex_handle_is_the_domain() {
		let rendered = rendered(
			Stack::new("atproto"),
			&[("example.com", COMPANY), ("pete.example.com", ALICE)],
			|block| block,
		);
		rendered
			.iter()
			.map(|(label, name, _)| (label.as_str(), name.as_str()))
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				(
					"atproto__dev__example_com_atproto_apex",
					"_atproto.example.com",
				),
				(
					"atproto__dev__example_com_atproto_pete",
					"_atproto.pete.example.com",
				),
			]);
	}

	/// A handle record is a real global name: two stages publishing the same
	/// declaration would put a second TXT at it and the handle would resolve to
	/// neither. Only the owning stage may publish.
	#[beet_core::test]
	fn only_the_owning_stage_publishes() {
		let accounts = [("alice.example.com", ALICE)];
		let owned = |block: AtprotoHandleBlock| block.with_dns_stage("prod");
		rendered(Stack::new("atproto").with_stage("prod"), &accounts, owned)
			.len()
			.xpect_eq(1);
		rendered(Stack::new("atproto").with_stage("drill"), &accounts, owned)
			.xpect_eq(Vec::new());
	}

	/// A typo is caught before any record exists, since the alternative is
	/// discovering it as an account whose handle the app calls invalid.
	#[beet_core::test]
	fn invalid_declarations_fail_at_render() {
		render_err(&[("www.example.com", ALICE)])
			.xpect_contains("reserved hostname");
		render_err(&[("alice.other.com", ALICE)])
			.xpect_contains("not under 'example.com'");
		render_err(&[("a.b.example.com", ALICE)]).xpect_contains("handle name");
		render_err(&[("alice.example.com", ALICE), ("Alice.example.com", BOB)])
			.xpect_contains("declared twice");
		// `apex` is a real label as well as the apex's sentinel, so both is
		// two records at one terraform label, refused rather than replaced
		render_err(&[("example.com", COMPANY), ("apex.example.com", ALICE)])
			.xpect_contains("duplicate resource");
	}

	/// The block authors from markup, its accounts named by `bx:ref`.
	///
	/// A block whose type does not register resolves to NOTHING at all, so an
	/// entry declaring one would build a stack with no records in it and
	/// deploy successfully. That failure mode is the reason this test exists
	/// rather than a construction test.
	#[beet_core::test]
	fn spawns_by_tag() {
		let mut world =
			(AsyncPlugin, TemplatePlugin, DocumentPlugin, InfraPlugin)
				.into_world();
		world
			.spawn(())
			.insert_template(
				BsxTemplate::parse_document(&format!(
					r#"<Fragment>
					<AtprotoAccount bx:ref="company" handle="example.com" did="{COMPANY}"/>
					<AtprotoAccount bx:ref="alice" handle="alice.example.com" did="{ALICE}"/>
					<AtprotoHandleBlock domain="example.com" dns_stage="prod"
						accounts={{[$company, $alice]}}/>
					<AtprotoHandleProbe/>
				</Fragment>"#
				))
				.unwrap(),
			)
			.unwrap();
		world.flush();
		let block = world
			.query::<&AtprotoHandleBlock>()
			.single(&world)
			.unwrap()
			.clone();
		block.accounts().len().xpect_eq(2);
		world
			.run_system_once(move |accounts: Query<&AtprotoAccount>| {
				block
					.handles(&accounts)
					.unwrap()
					.iter()
					.map(|handle| handle.handle("example.com"))
					.collect::<Vec<_>>()
			})
			.unwrap()
			.xpect_eq(vec!["example.com", "alice.example.com"]);
		world
			.query::<&AtprotoHandleProbe>()
			.single(&world)
			.unwrap()
			.appview
			.as_str()
			.xpect_eq(HandleResolver::PUBLIC_APPVIEW);
	}
}

//! The declared mail stack, as every post-apply step reads it.
use crate::prelude::*;
use beet_core::prelude::*;

/// Everything a mail deploy step needs about the stack it runs in: the tofu
/// project holding the apply's outputs, the box those outputs describe, and the
/// domains it serves.
///
/// One resolution shared by every step, so `EipReverseDns`, `StalwartProvision`
/// and `MailProbe` cannot disagree about which box they are talking to. A step
/// that reached for the blocks itself would be free to find a different one.
pub struct MailStack {
	/// The applied project, for its outputs and its work directory.
	pub project: terra::Project,
	/// The identity every name in the stack composes from.
	pub stack: ResolvedStack,
	/// The one box under this stack.
	pub mail_box: StalwartBlock,
	/// The database the box's [`DatabaseRef`] targets, ie the one holding its
	/// mail metadata, whose compositions every step reads.
	pub database: RdsPostgresBlock,
	/// Every domain it serves, in declaration order.
	pub domains: Vec<MailDomainBlock>,
	/// The relay each of those domains resolved, by the same ancestry the
	/// render used. Resolved once here so a probe, a plan and a credential
	/// listing cannot disagree about which provider carries a domain.
	pub relays: RelayModes,
}

impl MailStack {
	/// Render the stack `entity` belongs to and resolve the whole mail stack
	/// from it: the one entry every mail verb reaches the stack through, shaped
	/// to pass directly to [`AsyncEntity::with_world`].
	pub fn resolve(world: &mut World, entity: Entity) -> Result<MailStack> {
		let project = RenderScope::render(world, entity)?.project()?;
		world.with_state::<MailQuery, _>(|query| query.resolve(entity, project))
	}

	/// The address of the box, from the apply's output.
	pub async fn public_ip(&self) -> Result<String> {
		self.project
			.output(&format!("{}_public_ip", self.mail_box.label()))
			.await
	}

	/// The address of the database this box's mail lives in, from the apply's
	/// output.
	///
	/// Read from the output rather than from [`RdsPostgresBlock::host`], which
	/// composes a terraform REFERENCE: the right value in a config file
	/// terraform interpolates, and a literal `${aws_db_instance..}` anywhere
	/// else. Anything that reaches the database over ssh — the restore, a
	/// manual dump — needs the resolved name, and asking the project for it is
	/// the only way to be sure the two agree.
	pub async fn database_host(&self) -> Result<String> {
		let endpoint = self
			.project
			.output(&format!("{}_endpoint", self.database.label()))
			.await?;
		// `endpoint` is `host:port` and every caller names the port itself
		endpoint
			.split(':')
			.next()
			.unwrap_or(endpoint.as_str())
			.to_string()
			.xok()
	}

	/// The EIP allocation id, ie what a reverse-dns request names.
	pub async fn eip_allocation(&self) -> Result<String> {
		self.project
			.output(&format!("{}_eip_allocation", self.mail_box.label()))
			.await
	}

	/// The relay `domain` leaves through.
	pub fn relay(&self, domain: &MailDomainBlock) -> &RelayMode {
		self.relays.get(domain.domain())
	}

	/// Every served domain paired with its relay, ie what a step that behaves
	/// differently per provider iterates.
	pub fn relayed(
		&self,
	) -> impl Iterator<Item = (&MailDomainBlock, &RelayMode)> {
		self.serving().map(|domain| (domain, self.relay(domain)))
	}

	/// The domains this box actually serves, ie the ones whose records hand
	/// their mail here, in declaration order.
	///
	/// An [`IdentityOnly`](MailRecords::IdentityOnly) domain is deliberately
	/// absent: it is a cutover prepared ahead of its window, so its identity
	/// signs and its selectors verify while the server must not hold it as a
	/// local domain. A local domain hijacks every submission addressed to it
	/// away from the MX the world still resolves, and its autoconfig host
	/// resolves nowhere, which is enough to kill an ACME order for every name
	/// beside it.
	pub fn serving(&self) -> impl Iterator<Item = &MailDomainBlock> {
		self.domains
			.iter()
			.filter(|domain| domain.records().serves_mail())
	}

	/// The domain declaring a mailbox at `localpart`, which is how a step names
	/// a mailbox without restating which domain holds it. Exactly one, since
	/// two would make `probe@` ambiguous.
	pub fn domain_holding(&self, localpart: &str) -> Result<&MailDomainBlock> {
		let mut found = self.domains.iter().filter(|domain| {
			domain
				.mailboxes()
				.iter()
				.any(|mailbox| mailbox.localpart() == localpart)
		});
		let domain = found.next().ok_or_else(|| {
			bevyhow!(
				"no mail domain under this stack declares a '{localpart}' mailbox"
			)
		})?;
		if found.next().is_some() {
			bevybail!(
				"several mail domains declare a '{localpart}' mailbox, so the \
				address is ambiguous"
			);
		}
		Ok(domain)
	}

	/// The domain named, ie the sending domain a probe's inbound leg comes
	/// from. Named rather than guessed: which domain sends is a deliberate
	/// choice, not the first one declared.
	pub fn domain_named(&self, name: &str) -> Result<&MailDomainBlock> {
		self.domains
			.iter()
			.find(|domain| domain.domain() == name)
			.ok_or_else(|| {
				bevyhow!(
					"'{name}' is not a mail domain under this stack; declared: {}",
					self.domains
						.iter()
						.map(|domain| domain.domain().to_string())
						.collect::<Vec<_>>()
						.join(", ")
				)
			})
	}
}

/// The deploy tree a mail step reads: the stack traversal, the block types the
/// mail stack is made of, and the box's [`DatabaseRef`] relation.
#[derive(SystemParam)]
pub struct MailQuery<'w, 's> {
	stacks: StackQuery<'w, 's>,
	boxes:
		Query<'w, 's, (&'static StalwartBlock, Option<&'static DatabaseRef>)>,
	databases: Query<'w, 's, &'static RdsPostgresBlock>,
	domains: Query<'w, 's, &'static MailDomainBlock>,
	relays: RelayQuery<'w, 's>,
}

impl MailQuery<'_, '_> {
	/// Resolve the whole mail stack from any entity declared under it, with
	/// the `project` a [`RenderScope`] rendered (see [`MailStack::resolve`],
	/// which composes both).
	///
	/// Several boxes under one stack is an error rather than a guess: they
	/// would have different hostnames, different certificates and different
	/// admin credentials, and picking one silently would provision the wrong
	/// server.
	pub fn resolve(
		&self,
		entity: Entity,
		project: terra::Project,
	) -> Result<MailStack> {
		let (_, stack) = self.stacks.root(entity)?;
		let declared = self.stacks.declared(entity)?;
		let mut boxes = declared
			.iter()
			.filter_map(|child| self.boxes.get(*child).ok());
		let (mail_box, database_ref) = boxes.next().ok_or_else(|| {
			bevyhow!(
				"no StalwartBlock is declared under this stack, so there is \
				no mail box to provision"
			)
		})?;
		let mail_box = mail_box.clone();
		if boxes.next().is_some() {
			bevybail!(
				"several StalwartBlocks are declared under this stack, so a \
				mail step cannot tell which box it is talking to"
			);
		}
		// the box's database, through the same relation its render resolves
		let database_ref = database_ref.ok_or_else(|| {
			bevyhow!(
				"the mail box '{}' declares no `DatabaseRef`: relate it to the \
				RdsPostgresBlock holding mail metadata, ie `{{DatabaseRef($db)}}`",
				mail_box.label()
			)
		})?;
		let database = self
			.databases
			.get(database_ref.0)
			.map_err(|_| {
				bevyhow!(
					"the `DatabaseRef` of '{}' targets no RdsPostgresBlock",
					mail_box.label()
				)
			})?
			.clone();
		let mut relays = RelayModes::default();
		let mut domains = Vec::new();
		for child in declared.iter() {
			let Ok(domain) = self.domains.get(*child) else {
				continue;
			};
			relays.insert(
				domain.domain().clone(),
				self.relays.resolve(*child, domain.domain())?,
			);
			domains.push(domain.clone());
		}
		if domains.is_empty() {
			bevybail!(
				"no MailDomainBlock is declared under this stack, so the box \
				would serve no domain at all"
			);
		}
		Ok(MailStack {
			project,
			stack,
			mail_box,
			database,
			domains,
			relays,
		})
	}
}

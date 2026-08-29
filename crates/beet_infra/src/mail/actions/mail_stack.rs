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
	/// Every domain it serves, in declaration order.
	pub domains: Vec<MailDomainBlock>,
}

impl MailStack {
	/// The address of the box, from the apply's output.
	pub async fn public_ip(&self) -> Result<String> {
		self.project
			.output(&format!("{}_public_ip", self.mail_box.label()))
			.await
	}

	/// The address of the database this box's mail lives in, from the apply's
	/// output.
	///
	/// Read from the output rather than from [`DatabaseRef::host`], which
	/// composes a terraform REFERENCE: the right value in a config file
	/// terraform interpolates, and a literal `${aws_db_instance..}` anywhere
	/// else. Anything that reaches the database over ssh — the restore, a
	/// manual dump — needs the resolved name, and asking the project for it is
	/// the only way to be sure the two agree.
	pub async fn database_host(&self) -> Result<String> {
		let endpoint = self
			.project
			.output(&format!(
				"{}_endpoint",
				self.mail_box.database().label()
			))
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

/// The deploy tree a mail step reads: the stack traversal, plus the two block
/// types the mail stack is made of.
#[derive(SystemParam)]
pub struct MailQuery<'w, 's> {
	stacks: StackQuery<'w, 's>,
	boxes: Query<'w, 's, &'static StalwartBlock>,
	domains: Query<'w, 's, &'static MailDomainBlock>,
}

impl MailQuery<'_, '_> {
	/// Resolve the whole mail stack from any entity declared under it.
	///
	/// Several boxes under one stack is an error rather than a guess: they
	/// would have different hostnames, different certificates and different
	/// admin credentials, and picking one silently would provision the wrong
	/// server.
	pub fn resolve(&self, entity: Entity) -> Result<MailStack> {
		let project = self.stacks.build_project(entity)?;
		let (_, stack) = self.stacks.root(entity)?;
		let declared = self.stacks.declared(entity)?;
		let mut boxes = declared
			.iter()
			.filter_map(|child| self.boxes.get(*child).ok());
		let mail_box = boxes
			.next()
			.ok_or_else(|| {
				bevyhow!(
					"no StalwartBlock is declared under this stack, so there is \
					no mail box to provision"
				)
			})?
			.clone();
		if boxes.next().is_some() {
			bevybail!(
				"several StalwartBlocks are declared under this stack, so a \
				mail step cannot tell which box it is talking to"
			);
		}
		let domains = declared
			.iter()
			.filter_map(|child| self.domains.get(*child).ok())
			.cloned()
			.collect::<Vec<_>>();
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
			domains,
		})
	}
}

//! Reading back the credentials the stack generated but never showed anyone.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `<MailCredentials/>` — print every mailbox credential this stack holds, as
/// the address that uses it.
///
/// Nothing in this stack ever shows a password: `StalwartProvision` mints each
/// mailbox credential, parks it in parameter store and moves on, and the
/// server mints its own administrator's. That is the right default, and it
/// leaves one real gap — a human setting up a mail client needs the value, and
/// the honest alternative to this verb is a hand-composed `aws ssm
/// get-parameter` per mailbox with the parameter name typed from memory. The
/// name is composed by [`AccountPlan::secret_ref`], so a verb that reads it
/// back cannot disagree with the step that wrote it, and a mailbox added to the
/// declaration appears here without anything else being updated.
///
/// This DELIBERATELY prints secrets to stdout, which is the one place in the
/// mail stack that does. Everything else redacts, so treat the output the way
/// the parameter store treats the value: it reaches a terminal, a scrollback
/// buffer and whatever is recording the session.
///
/// `--infra` adds the credentials no human signs in with: the database master
/// password, the SES SMTP pair and the DKIM signing keys. Separate because
/// reading a mailbox password is setting up a mail client, and reading the
/// database password is an incident.
#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(MailCredentialsAction)]
pub struct MailCredentials;

/// Reads each parameter and prints it beside the address it belongs to.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ParamsPartial = ParamsPartial::new::<MailCredentialsParams>())]
pub async fn MailCredentialsAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let infra = cx.has_param("infra");
	let mail = cx
		.caller
		.with_state::<MailQuery, _>(|entity, query| query.resolve(entity))
		.await??;
	let region = mail.stack.region().clone();
	let label = mail.mail_box.label();

	// the server's own administrator first, since it is the one account no
	// declaration names: the `Bootstrap` claim created it on the FIRST domain
	// served and parked its credential beside the mailbox ones.
	let mut entries = Vec::new();
	if let Some(domain) = mail.serving().next() {
		entries.push((
			format!("{}@{}", StalwartBlock::ADMIN_USER, domain.domain()),
			AccountPlan::secret_ref(
				label,
				StalwartBlock::ADMIN_USER,
				&domain.slug(),
			),
			format!("administers {}", mail.mail_box.hostname()),
		));
	}
	// only the served domains: provision mints a credential per mailbox it
	// creates, and it creates none on a domain the server does not hold.
	for domain in mail.serving() {
		for mailbox in domain.mailboxes() {
			entries.push((
				format!("{}@{}", mailbox.localpart(), domain.domain()),
				AccountPlan::secret_ref(
					label,
					mailbox.localpart(),
					&domain.slug(),
				),
				match mailbox.admin() {
					true => "mailbox, administrator".to_string(),
					false => "mailbox".to_string(),
				},
			));
		}
	}
	if infra {
		entries.push((
			format!("{} database", mail.mail_box.db_name()),
			mail.mail_box.database().secret(),
			"postgres master password".to_string(),
		));
		// keys exist exactly where `EnsureDkimKey` mints them, ie wherever the
		// records prove the identity, which includes a cutover-staged domain.
		for domain in mail
			.domains
			.iter()
			.filter(|domain| domain.records().proves_identity())
		{
			entries.push((
				format!("{} dkim key", domain.domain()),
				domain.dkim_secret(),
				format!(
					"private half of {}._domainkey",
					MailDomainBlock::DKIM_SELECTOR
				),
			));
		}
	}

	cross_log!("");
	for (name, secret, note) in entries {
		let parameter = secret.name(&mail.stack);
		match ssm_ext::get(&region, &parameter).await? {
			Some(value) => {
				cross_log!("{name}\n  {value}\n  {parameter} ({note})\n")
			}
			// a mailbox declared but never provisioned, which is a real and
			// readable state rather than a failure: the next deploy mints it.
			None => cross_log!(
				"{name}\n  (not minted yet)\n  {parameter} ({note})\n"
			),
		}
	}
	if !infra {
		info!(
			"pass --infra for the credentials no human signs in with (database, dkim)"
		);
	}
	Pass(cx.input).xok()
}

/// Parameters for the listing.
#[derive(Reflect)]
struct MailCredentialsParams {
	/// Also print the database master password and the DKIM signing keys.
	infra: bool,
}

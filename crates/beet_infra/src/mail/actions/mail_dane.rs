//! The DANE pin: the key port 25 serves, read off the box and parked for the
//! apply that publishes it.
use crate::actions::ssm_ext;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

impl MailDane {
	/// The digest a `3 1 1` record carries, from a PEM certificate on stdin:
	/// the SHA-256 of the leaf's SubjectPublicKeyInfo, DER, as 64 hex chars.
	///
	/// One pipeline for the box and the deploy machine, so what is parked and
	/// what [`MailHealth`] later compares against are computed identically.
	pub const SPKI_SHA256: &'static str = "openssl x509 -pubkey -noout | \
		openssl pkey -pubin -outform DER | sha256sum | cut -c1-64";

	/// What runs on the box: the pin port 25 presents to a peer that names the
	/// box in SNI and to one that names nothing, one digest per line.
	///
	/// Loopback, because the box cannot dial its own public address on 25 (EC2
	/// blocks the port as egress) and nor can most deploy machines. Both
	/// handshakes rather than one because a validating sender may send either,
	/// and a server whose default certificate is another domain's answers the
	/// second with a key the record does not pin.
	pub fn command(hostname: &str) -> String {
		[
			format!("-servername {hostname}"),
			"-noservername".to_string(),
		]
		.iter()
		.map(|sni| {
			format!(
				"echo | openssl s_client -connect 127.0.0.1:{} -starttls \
					smtp {sni} 2>/dev/null | {}",
				MailHealth::SMTP_PORT,
				Self::SPKI_SHA256
			)
		})
		.collect::<Vec<_>>()
		.join("; ")
	}

	/// The one pin both handshakes agreed on, or why there is none to publish.
	pub fn pin_from_output(output: &str) -> Result<String> {
		let digests = output
			.lines()
			.map(str::trim)
			.filter(|line| !line.is_empty())
			.collect::<Vec<_>>();
		let [with_sni, without_sni] = digests.as_slice() else {
			bevybail!(
				"port 25 did not complete both STARTTLS handshakes (got {:?}): \
				no pin is published until it does",
				digests
			);
		};
		if !Self::is_digest(with_sni) || !Self::is_digest(without_sni) {
			bevybail!(
				"port 25 presented no parseable certificate (got {:?})",
				digests
			);
		}
		if with_sni != without_sni {
			bevybail!(
				"port 25 serves one key with SNI ({with_sni}) and another \
				without ({without_sni}): a validating sender that names nothing \
				would be refused, so no pin is published until the default \
				certificate is the box's own (provision converges it)"
			);
		}
		with_sni.to_string().xok()
	}

	fn is_digest(value: &str) -> bool {
		value.len() == 64 && value.chars().all(|char| char.is_ascii_hexdigit())
	}
}

/// Reads the served key off the box and parks its digest where the apply
/// reads it.
/// `<MailDane/>` — pin the certificate the box serves, after provision and
/// before the apply that publishes the pin.
///
/// A `TLSA` that does not match the served key is worse than none: a
/// DANE-validating sender refuses the session rather than falling back, and
/// keeps refusing until the record or the key changes. So the pin is never
/// typed and never trusted across a deploy. It is read off port 25 on the box
/// itself, after every provision (which is where a fresh data store gets its
/// first certificate), parked at [`StalwartBlock::tlsa_secret`], and published
/// by the `<TofuApply/>` that follows this step in the same deploy: a stack
/// whose key changed publishes the new pin before the deploy ends, and one
/// whose key did not applies a no-op.
///
/// The key survives renewal because provision sets the ACME provider to reuse
/// it, so in steady state this step reads the same digest every deploy and
/// writes nothing. A fresh stack has nothing to pin at the first apply; the
/// variable resolves empty, no record renders, and the second apply of that
/// same deploy publishes the pin this step has just parked.
#[action]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub async fn MailDane(
	/// The private half of the key pair the box imported, as
	/// [`StalwartProvision`] takes it.
	#[field(default = StalwartProvision::SSH_KEY)]
	ssh_key: SmolStr,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let mail = cx.caller.with_world(MailStack::resolve).await??;
	if !mail.mail_box.dane() {
		bevybail!(
			"mail box '{}' does not declare `dane=true`, so nothing would \
			publish the pin this step reads: declare it, or remove this step",
			mail.mail_box.label()
		);
	}
	let region = mail.stack.region().clone();
	let hostname = mail.mail_box.hostname().to_string();
	let connection = SshConnection {
		host: mail.public_ip().await?,
		user: StalwartProvision::SSH_USER.to_string(),
		port: 22,
		key_path: StalwartProvision::key_path(&ssh_key)?,
	};
	connection
		.wait_for_ready(Duration::from_secs(120), Duration::from_secs(5))
		.await?;
	let output = connection
		.run_command(&MailDane::command(&hostname))
		.await?;
	let pin =
		MailDane::pin_from_output(&String::from_utf8_lossy(&output.stdout))?;

	let name = mail.mail_box.tlsa_secret().name(&mail.stack);
	let (usage, selector, matching) = StalwartBlock::TLSA_PARAMS;
	match ssm_ext::get(&region, &name).await?.as_deref() {
		Some(parked) if parked == pin => info!(
			"{} pins {usage} {selector} {matching} {pin}, unchanged",
			mail.mail_box.tlsa_record_name()
		),
		parked => {
			ssm_ext::overwrite(&region, &name, &pin).await?;
			info!(
				"{} pins {usage} {selector} {matching} {pin} ({}); the apply \
				after this publishes it",
				mail.mail_box.tlsa_record_name(),
				match parked {
					Some(_) => "the served key changed",
					None => "first pin",
				}
			);
		}
	}
	Pass(cx.input).xok()
}

#[cfg(test)]
mod tests {
	use super::*;

	const DIGEST: &str =
		"f0d32477eb8342450c31bd64079102915e01208afb51ed8883b3a9e13e2405ec";

	/// The pin is what BOTH handshakes present. Two different keys is the
	/// live finding this step exists to refuse: a server with no default
	/// certificate answers a peer without SNI with another domain's.
	#[beet_core::test]
	fn both_handshakes_must_agree() {
		MailDane::pin_from_output(&format!("{DIGEST}\n{DIGEST}\n"))
			.unwrap()
			.as_str()
			.xpect_eq(DIGEST);
		MailDane::pin_from_output(&format!(
			"{DIGEST}\n{}\n",
			DIGEST.replace('f', "0")
		))
		.unwrap_err()
		.to_string()
		.xpect_contains("one key with SNI");
		// a handshake that produced nothing is not a pin of nothing
		MailDane::pin_from_output(&format!("{DIGEST}\n"))
			.unwrap_err()
			.to_string()
			.xpect_contains("both STARTTLS handshakes");
		MailDane::pin_from_output(
			"unable to load certificate\nunable to load certificate\n",
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("no parseable certificate");
	}

	/// Read on loopback, once with SNI and once without, through the one
	/// digest pipeline the health check also runs.
	#[beet_core::test]
	fn the_pin_is_read_on_the_box_both_ways() {
		let command = MailDane::command("mail.beetmash.com");
		command
			.as_str()
			.xpect_contains("127.0.0.1:25")
			.xpect_contains("-servername mail.beetmash.com")
			.xpect_contains("-noservername")
			.xpect_contains(MailDane::SPKI_SHA256);
		command.matches("-starttls smtp").count().xpect_eq(2);
	}
}

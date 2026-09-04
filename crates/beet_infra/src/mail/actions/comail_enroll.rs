//! Checking that the enrolment a human did at comail is the one this stack
//! believes in.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;

/// `<ComailEnroll/>`: verify every comail-relayed domain's enrolment, and hand
/// the records it minted to the apply.
///
/// Enrolment itself is NOT automated and cannot be: both endpoints are gated on
/// an atproto OAuth session cookie whose DID must match the claim
/// (`internal/admin/enroll_start_phases.go:64-79`), and there is no api-key path
/// beside it. So a human enrols the domain in a browser, parks the five values
/// the response carries in parameter store, and this verb is what turns that
/// into something a deploy can fail on:
///
/// 1. every parameter exists, else it fails naming the missing one and what it
///    holds;
/// 2. the api key authenticates, checked with the one member-facing endpoint
///    that takes one (`GET /member/deliverability`), which is also the endpoint
///    the observability job polls;
/// 3. the two selector records resolve in dns, reported rather than failed,
///    since on a first deploy they are published by the apply that follows.
///
/// It runs BEFORE `<TofuApply/>`, for the same reason `<EnsureDkimKey/>` does:
/// the selector records read their values out of tofu variables, and a variable
/// nobody supplied resolves to its empty default, which for a DKIM record is
/// the wire form of a REVOKED selector rather than a missing one.
///
/// A stack with no comail domain passes trivially, so the step is safe to leave
/// in a route whose domains later move to another relay.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ComailEnrollAction)]
pub struct ComailEnroll {
	/// The resolver the selector check asks, over DNS-over-HTTPS so the check
	/// needs no resolver library and behaves the same on every target.
	resolver: SmolStr,
}

impl Default for ComailEnroll {
	fn default() -> Self {
		Self {
			resolver: Self::RESOLVER.into(),
		}
	}
}

impl ComailEnroll {
	/// The DoH endpoint the record check queries.
	pub const RESOLVER: &'static str = "https://cloudflare-dns.com/dns-query";

	/// The `TXT` values at `name`, asked over DoH.
	///
	/// A resolver over HTTP rather than a system lookup because a `TXT` query
	/// is not something `std` does and the deploy is target-agnostic: the same
	/// request works from a laptop, a lambda and a browser.
	async fn lookup_txt(&self, name: &str) -> Result<Vec<String>> {
		let response =
			Request::get(format!("{}?name={name}&type=TXT", self.resolver))
				.with_header_raw("accept", "application/dns-json")
				.send()
				.await?;
		if !response.status().is_ok() {
			bevybail!("{} answered {}", self.resolver, response.status());
		}
		let json: Value = response.json().await?;
		json["Answer"]
			.as_array()
			.map(|answers| {
				answers
					.iter()
					.filter_map(|answer| answer["data"].as_str())
					// a DoH `TXT` answer arrives quoted, and a long value
					// arrives as several quoted strings to be concatenated
					.map(|data| data.replace('"', "").replace(' ', ""))
					.collect::<Vec<_>>()
			})
			.unwrap_or_default()
			.xok()
	}

	/// What to do about a domain whose parameters are not all parked: the
	/// missing names, what each holds, and the step that fills them.
	///
	/// Composed here rather than inline so the instructions and the names they
	/// print come off [`ComailRelay::secrets`], which is also what the
	/// credential read uses: a hand-written second copy of a parameter name is
	/// the drift this exists to prevent, and its failure mode is a `535` three
	/// minutes into a provision.
	pub fn missing_message(
		domain: &str,
		host: &str,
		region: &str,
		missing: &[String],
	) -> String {
		format!(
			"'{domain}' relays through comail but {} of its 5 parameters are \
			not parked:\n{}\n\nEnrol the domain at https://{host}, signing in \
			with the atproto account it belongs to, then park each value from \
			the response with `aws ssm put-parameter --type SecureString \
			--name <name> --value <value> --region {region}`. One domain per \
			DID, and a subdomain is a separate domain.",
			missing.len(),
			missing.join("\n")
		)
	}

	/// Whether the enrolled key authenticates, asked with the one endpoint that
	/// takes an api key rather than a session cookie.
	///
	/// A `4xx` here is the honest failure: the key is wrong, the DID is wrong,
	/// or the two do not belong together, and every one of them means the relay
	/// routes this deploy is about to write cannot send.
	async fn check_credential(
		comail: &ComailRelay,
		did: &str,
		api_key: &str,
	) -> Result<Value> {
		let response = Request::get(comail.deliverability_url(did))
			.with_auth_bearer(api_key)
			.send()
			.await?;
		let status = response.status();
		let body = response.text().await.unwrap_or_default();
		if !status.is_ok() {
			bevybail!(
				"comail refused the parked credential ({status}): {body}. The \
				DID and the api key must be the pair one enrolment returned, \
				and the key belongs to the one domain it enrolled"
			);
		}
		serde_json::from_str(&body).map_err(Into::into)
	}
}

/// Checks every comail domain's parameters, credential and records, and passes
/// the minted record values on to the apply as `-var`s.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn ComailEnrollAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let enroll = cx
		.caller
		.get_cloned::<ComailEnroll>()
		.await
		.unwrap_or_default();
	let mail = cx.caller.with_world(MailStack::resolve).await??;
	let region = mail.stack.region().clone();

	let mut input = cx.input;
	let mut checked = 0usize;
	for (domain, relay) in mail.relayed() {
		let RelayMode::Comail(comail) = relay else {
			continue;
		};
		checked += 1;
		let slug = domain.slug();
		// every parameter first, so a half-parked enrolment reports all of what
		// is missing rather than the first one
		let mut values = Vec::new();
		let mut missing = Vec::new();
		for (secret, holds) in ComailRelay::secrets(&slug) {
			let name = secret.name(&mail.stack);
			match ssm_ext::get(&region, &name).await? {
				Some(value) if !value.is_empty() => values.push(value),
				_ => missing.push(format!("  {name}  ({holds})")),
			}
		}
		if !missing.is_empty() {
			bevybail!(
				"{}",
				ComailEnroll::missing_message(
					domain.domain(),
					comail.host(),
					&region,
					&missing
				)
			);
		}
		let [did, api_key, selector, rsa_record, ed_record] =
			<[String; 5]>::try_from(values)
				.map_err(|_| bevyhow!("ComailRelay::secrets changed shape"))?;

		let status =
			ComailEnroll::check_credential(comail, &did, &api_key).await?;
		info!(
			"comail accepts '{}' as {did} ({}, {} sent in 14d)",
			domain.domain(),
			status["status"].as_str().unwrap_or("unknown"),
			status["sent_14d"].as_i64().unwrap_or_default()
		);

		// the records the apply publishes read these out of variables, exactly
		// as the sovereign selector reads its own minted key
		for (variable, value) in [
			(ComailRelay::dkim_selector_variable(&slug), &selector),
			(ComailRelay::dkim_rsa_variable(&slug), &rsa_record),
			(ComailRelay::dkim_ed_variable(&slug), &ed_record),
		] {
			input = input.with_param(variable.key(), value);
		}

		check_records(&enroll, domain, comail, &selector, &rsa_record).await;
	}
	match checked {
		0 => info!("no mail domain relays through comail"),
		count => info!("{count} comail enrolment(s) verified"),
	}
	Pass(input).xok()
}

/// Report on the records comail expects to see published: its RSA selector and
/// the apex SPF that includes the relay.
///
/// A warning rather than a failure, because this step runs BEFORE the apply
/// that publishes them: on a first deploy their absence is the normal state,
/// and failing here would make the step impossible to satisfy. On every deploy
/// after, a warning here is the thing to look at.
async fn check_records(
	enroll: &ComailEnroll,
	domain: &MailDomainBlock,
	comail: &ComailRelay,
	selector: &str,
	rsa_record: &str,
) {
	let name = format!("{selector}r._domainkey.{}", domain.domain());
	let expected = rsa_record.replace('"', "").replace(' ', "");
	let checks = [
		(name, expected, "the RSA selector comail signs with"),
		(
			domain.domain().to_string(),
			comail.spf_value().replace(' ', ""),
			"the apex SPF authorising the relay",
		),
	];
	for (name, expected, what) in checks {
		match enroll.lookup_txt(&name).await {
			Ok(values) if values.iter().any(|value| value == &expected) => {
				info!("{name} resolves: {what}")
			}
			Ok(_) => warn!(
				"{name} does not resolve to {what} yet; the apply after this \
				step publishes it, so this is expected on a first deploy and \
				is where to look on any other"
			),
			// a resolver that will not answer says nothing about the zone
			Err(err) => warn!("could not resolve {name}: {err}"),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The five parameters are one list, so the check, the instructions it
	/// prints and the credential read cannot disagree about what a domain
	/// needs. Their names carry the slug, since a stack may enrol several
	/// domains and comail issues a key per domain.
	#[beet_core::test]
	fn every_parameter_is_named_once_and_per_domain() {
		let stack = Stack::new("beetmash")
			.with_stage("prod")
			.resolve(&PackageConfig::default());
		ComailRelay::secrets("news-example-com")
			.into_iter()
			.map(|(secret, _)| secret.name(&stack))
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"/beetmash/prod/comail-did-news-example-com",
				"/beetmash/prod/comail-api-key-news-example-com",
				"/beetmash/prod/comail-dkim-selector-news-example-com",
				"/beetmash/prod/comail-dkim-rsa-news-example-com",
				"/beetmash/prod/comail-dkim-ed-news-example-com",
			]);
	}

	/// A half-parked enrolment names every parameter it is missing, what each
	/// one holds and the step that fills it, because the alternative is a
	/// provision that authenticates as nobody and reports a `535` three
	/// minutes later.
	#[beet_core::test]
	fn a_missing_parameter_names_itself_and_the_enrolment() {
		let stack = Stack::new("beetmash")
			.with_stage("prod")
			.resolve(&PackageConfig::default());
		let missing = ComailRelay::secrets("news-example-com")
			.into_iter()
			.map(|(secret, holds)| {
				format!("  {}  ({holds})", secret.name(&stack))
			})
			.collect::<Vec<_>>();
		ComailEnroll::missing_message(
			"news.example.com",
			ComailRelay::HOST,
			"us-west-2",
			&missing[..2],
		)
		.as_str()
		.xpect_contains("'news.example.com' relays through comail")
		.xpect_contains("/beetmash/prod/comail-did-news-example-com")
		.xpect_contains("the enrolled DID")
		.xpect_contains("Enrol the domain at https://smtp.atmos.email")
		.xpect_contains("aws ssm put-parameter")
		.xpect_contains("--region us-west-2")
		.xpect_contains("a subdomain is a separate domain");
	}

	/// A DoH `TXT` answer arrives quoted and, over 255 bytes, split into
	/// several quoted strings meant to be concatenated. A comparison against
	/// the raw answer would fail on every RSA key, which is every key comail
	/// mints.
	#[beet_core::test]
	fn a_quoted_split_txt_answer_reads_as_one_value() {
		let value = "\"v=DKIM1; k=rsa; p=MIIBIjAN\" \"AQEFAAOC\"";
		value
			.replace('"', "")
			.replace(' ', "")
			.as_str()
			.xpect_eq("v=DKIM1;k=rsa;p=MIIBIjANAQEFAAOC");
	}
}

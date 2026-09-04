//! The scheduled read of what comail thinks of our sending.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;

/// `<ComailDeliverability/>`: poll each comail domain's deliverability
/// aggregates and publish them as CloudWatch metrics.
///
/// The SES arm gets its numbers for free: the configuration set publishes
/// reputation metrics into `AWS/SES` and an alarm reads them. Comail has no
/// such stream on our side, and the two surfaces it does have are not
/// equivalent. Per-event webhooks carry the detail but their registration is
/// session-cookie gated with api-key auth explicitly deferred
/// (`internal/admin/ui/account_api_webhooks.go:6-17`), so nothing a deploy runs
/// can subscribe to them. `GET /member/deliverability` carries aggregates and
/// takes an api key, so it is the surface that can be automated, and aggregates
/// are what the question "is my mail healthy" actually wants.
///
/// This is what makes a comail domain's alarms mean anything: they read metrics
/// this job writes, so a comail stack without it is a stack whose first
/// complaint arrives as an auto-pause (5% bounce or 0.08% complaint over a
/// rolling 24h, `internal/relay/domain_pause_evaluator.go`). Declared as the
/// route a `<ScheduledJobBlock/>` invokes, not as a deploy step: a number read
/// once at deploy time is a number that was true once.
#[derive(Debug, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ComailDeliverabilityAction)]
pub struct ComailDeliverability {
	/// Publish the numbers even when a poll finds a domain paused or
	/// suspended. On by default, because a paused domain is exactly when the
	/// metrics matter; off for a stack that would rather the alarm not
	/// re-notify every half hour.
	report_paused: bool,
}

impl Default for ComailDeliverability {
	fn default() -> Self {
		Self {
			report_paused: true,
		}
	}
}

impl ComailDeliverability {
	/// The member states that mean comail has stopped sending for a domain, ie
	/// what [`PAUSED_METRIC`](ComailRelay::PAUSED_METRIC) reports as `1`.
	///
	/// Matched case-insensitively on a prefix, since the string is comail's own
	/// status column and a future `paused_bounce_rate` should still read as
	/// paused rather than as healthy.
	pub const STOPPED: &'static [&'static str] = &["paused", "suspended"];

	/// The metrics one deliverability response maps to.
	///
	/// The complaint RATE is derived here rather than read, because the
	/// response carries `complaints_14d` as a count and comail's own pause
	/// threshold is a rate: alarming on a count would fire on a busy fortnight
	/// and stay silent on a small one with a terrible ratio.
	pub fn metrics(&self, response: &Value) -> Vec<MetricDatum> {
		let number = |key: &str| response[key].as_f64().unwrap_or_default();
		let sent = number("sent_14d");
		let complaints = number("complaints_14d");
		let mut metrics = vec![
			MetricDatum::count(ComailRelay::SENT_METRIC, sent),
			MetricDatum::rate(
				ComailRelay::BOUNCE_RATE_METRIC,
				number("bounce_rate"),
			),
			MetricDatum::rate(ComailRelay::COMPLAINT_RATE_METRIC, match sent {
				0. => 0.,
				sent => complaints / sent,
			}),
		];
		if self.report_paused {
			metrics.push(MetricDatum::count(
				ComailRelay::PAUSED_METRIC,
				match Self::is_stopped(response) {
					true => 1.,
					false => 0.,
				},
			));
		}
		metrics
	}

	/// Whether the response reports a domain comail has stopped sending for.
	fn is_stopped(response: &Value) -> bool {
		let status = response["status"]
			.as_str()
			.unwrap_or_default()
			.to_ascii_lowercase();
		Self::STOPPED
			.iter()
			.any(|stopped| status.starts_with(stopped))
	}
}

/// Polls every comail domain and publishes what it reads.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn ComailDeliverabilityAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let poll = cx
		.caller
		.get_cloned::<ComailDeliverability>()
		.await
		.unwrap_or_default();
	let mail = cx.caller.with_world(MailStack::resolve).await??;
	let region = mail.stack.region().clone();

	for (domain, relay) in mail.relayed() {
		let RelayMode::Comail(comail) = relay else {
			continue;
		};
		let slug = domain.slug();
		let did = read(&mail, &region, ComailRelay::did_secret(&slug)).await?;
		let api_key =
			read(&mail, &region, ComailRelay::api_key_secret(&slug)).await?;

		let response = Request::get(comail.deliverability_url(&did))
			.with_auth_bearer(&api_key)
			.send()
			.await?;
		let status = response.status();
		let body = response.text().await.unwrap_or_default();
		if !status.is_ok() {
			bevybail!(
				"comail answered {status} for '{}': {body}",
				domain.domain()
			);
		}
		let response: Value = serde_json::from_str(&body)?;
		let metrics = poll.metrics(&response);
		cloudwatch_ext::put_metric_data(
			&region,
			ComailRelay::METRIC_NAMESPACE,
			ComailRelay::METRIC_DIMENSION,
			domain.domain(),
			&metrics,
		)
		.await?;
		info!(
			"{}: {} status, {} sent and {} bounced in 14d",
			domain.domain(),
			response["status"].as_str().unwrap_or("unknown"),
			response["sent_14d"].as_i64().unwrap_or_default(),
			response["bounced_14d"].as_i64().unwrap_or_default()
		);
	}
	Pass(cx.input).xok()
}

/// One of the enrolment parameters, failing with the step that fills it.
async fn read(
	mail: &MailStack,
	region: &str,
	secret: SecretRef,
) -> Result<String> {
	let name = secret.name(&mail.stack);
	ssm_ext::get(region, &name).await?.ok_or_else(|| {
		bevyhow!("{name} does not exist: run <ComailEnroll/>, which checks it")
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	/// The response comail actually returns
	/// (`internal/admin/api_member_self.go:222-231`).
	fn response(status: &str, sent: i64, complaints: i64) -> Value {
		json!({
			"did": "did:plc:example",
			"status": status,
			"sent_14d": sent,
			"bounced_14d": 3,
			"complaints_14d": complaints,
			"bounce_rate": 0.012,
			"daily_sends": [0, 0, 1],
			"hourly_limit": 50,
			"daily_limit": 500,
			"labels": [],
		})
	}

	/// The response's fields map onto the metrics the alarms read by name, and
	/// the complaint RATE is derived from the count the response carries: an
	/// alarm on a raw count fires on a busy fortnight and stays silent on a
	/// small one with a terrible ratio.
	#[beet_core::test]
	fn the_response_maps_onto_the_alarmed_metrics() {
		ComailDeliverability::default()
			.metrics(&response("active", 1000, 2))
			.into_iter()
			.map(|datum| (datum.name, datum.value))
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				("Sent14d".to_string(), 1000.),
				("BounceRate".to_string(), 0.012),
				("ComplaintRate".to_string(), 0.002),
				("Paused".to_string(), 0.),
			]);
	}

	/// A domain that has never sent has no rate at all, and must publish zero
	/// rather than a division by zero: `NaN` is rejected by `put-metric-data`,
	/// which would take the whole batch down with it.
	#[beet_core::test]
	fn a_domain_that_has_not_sent_publishes_zero() {
		ComailDeliverability::default()
			.metrics(&response("active", 0, 0))
			.into_iter()
			.find(|datum| datum.name == ComailRelay::COMPLAINT_RATE_METRIC)
			.unwrap()
			.value
			.xpect_eq(0.);
	}

	/// The state every other metric exists to arrive before, reported as a
	/// gauge so an alarm can fire on it. Matched on a prefix, so a future
	/// `paused_complaint_rate` reads as stopped rather than as healthy.
	#[beet_core::test]
	fn a_stopped_domain_reports_one() {
		let paused = |status: &str| {
			ComailDeliverability::default()
				.metrics(&response(status, 100, 0))
				.into_iter()
				.find(|datum| datum.name == ComailRelay::PAUSED_METRIC)
				.unwrap()
				.value
		};
		paused("active").xpect_eq(0.);
		paused("paused").xpect_eq(1.);
		paused("PAUSED_BOUNCE_RATE").xpect_eq(1.);
		paused("suspended").xpect_eq(1.);
	}

	/// The alarms fire under comail's own auto-pause thresholds, or the warning
	/// arrives after the pause it exists to pre-empt. Pinned against the
	/// shipped values (`internal/relay/domain_pause_evaluator.go:31-32`), so
	/// raising either alarm past them fails here rather than in production.
	#[beet_core::test]
	fn the_alarms_beat_the_auto_pause() {
		ComailRelay::PAUSE_BOUNCE_RATE.xpect_eq(0.05);
		ComailRelay::PAUSE_COMPLAINT_RATE.xpect_eq(0.0008);
		ComailRelay::BOUNCE_ALARM_RATE
			.xpect_less_than(ComailRelay::PAUSE_BOUNCE_RATE);
		ComailRelay::COMPLAINT_ALARM_RATE
			.xpect_less_or_equal_to(ComailRelay::PAUSE_COMPLAINT_RATE);
	}
}

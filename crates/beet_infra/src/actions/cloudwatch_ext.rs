//! Publishing custom metrics, over the `aws` cli.
//!
//! The cli for the same reason [`ssm_ext`](super::ssm_ext) uses it: every step
//! that would call this already shells out to `aws` for the parameters it reads
//! first, so an SDK client here would be a dependency bought for one verb.
use beet_core::prelude::*;
use serde_json::json;

/// One metric value, in a namespace's units.
///
/// A datum rather than three loose arguments because the three travel together
/// through the whole call and a transposed pair is a metric that reads as
/// healthy: the wrong number under the right name is invisible on a dashboard.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricDatum {
	/// The metric name, ie the `BounceRate` an alarm names.
	pub name: String,
	pub value: f64,
	/// The CloudWatch unit, ie `Percent` or `Count`. `None` is `None` on the
	/// wire too, which is what a dimensionless gauge wants.
	pub unit: Option<&'static str>,
}

impl MetricDatum {
	/// A rate, published as the fraction it is rather than as a percentage:
	/// the thresholds it is alarmed against are the provider's own, and those
	/// are fractions.
	pub fn rate(name: impl Into<String>, value: f64) -> Self {
		Self {
			name: name.into(),
			unit: Some("None"),
			value,
		}
	}

	/// A count of things.
	pub fn count(name: impl Into<String>, value: f64) -> Self {
		Self {
			name: name.into(),
			unit: Some("Count"),
			value,
		}
	}

	/// The wire form, as `put-metric-data --metric-data` takes it.
	pub fn to_json(
		&self,
		dimension: &str,
		dimension_value: &str,
	) -> serde_json::Value {
		json!({
			"MetricName": self.name,
			"Value": self.value,
			"Unit": self.unit.unwrap_or("None"),
			"Dimensions": [{ "Name": dimension, "Value": dimension_value }],
		})
	}
}

/// Publish `data` into `namespace`, every datum carrying the one
/// `(dimension, dimension_value)` pair.
///
/// One call for the whole batch rather than one per datum: they describe one
/// observation of one subject, so publishing them together is what keeps a
/// dashboard's rows aligned in time.
pub async fn put_metric_data(
	region: &str,
	namespace: &str,
	dimension: &str,
	dimension_value: &str,
	data: &[MetricDatum],
) -> Result {
	if data.is_empty() {
		return Ok(());
	}
	let metrics = data
		.iter()
		.map(|datum| datum.to_json(dimension, dimension_value))
		.collect::<Vec<_>>();
	ChildProcess::new("aws")
		.without_env("AWS_PROFILE")
		.with_args([
			"cloudwatch".to_string(),
			"put-metric-data".to_string(),
			"--namespace".to_string(),
			namespace.to_string(),
			"--metric-data".to_string(),
			serde_json::to_string(&metrics)?,
			"--region".to_string(),
			region.to_string(),
		])
		.run_async()
		.await?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The datum's wire shape is what an alarm's `dimensions` must match
	/// exactly: a metric published under a different dimension name is a
	/// metric no alarm ever reads, and an alarm on a metric that never appears
	/// sits in `INSUFFICIENT_DATA` looking exactly like a healthy one.
	#[beet_core::test]
	fn a_datum_carries_the_dimension_its_alarm_names() {
		MetricDatum::rate("BounceRate", 0.012)
			.to_json("Domain", "news.example.com")
			.to_string()
			.as_str()
			.xpect_eq(
				r#"{"Dimensions":[{"Name":"Domain","Value":"news.example.com"}],"MetricName":"BounceRate","Unit":"None","Value":0.012}"#,
			);
	}
}

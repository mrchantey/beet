//! Publishing custom metrics, as CloudWatch `PutMetricData`.
use beet_core::prelude::*;
use beet_net::prelude::*;

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

	/// The wire form, one entry of `PutMetricData`'s `MetricData`.
	pub fn to_json(&self, dimension: &str, dimension_value: &str) -> Value {
		value!({
			"MetricName": &self.name,
			"Value": self.value,
			"Unit": self.unit.unwrap_or("None"),
			"Dimensions": [{ "Name": dimension, "Value": dimension_value }],
		})
	}
}

/// Publish `data` into `namespace` in `region`, as the launch's own
/// credential, every datum carrying the one `(dimension, dimension_value)`
/// pair.
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
	put_metric_data_with(
		&AwsClient::new(AwsService::MONITORING, region),
		namespace,
		dimension,
		dimension_value,
		data,
	)
	.await
}

/// [`put_metric_data`] through `client`.
async fn put_metric_data_with(
	client: &AwsClient,
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
	client
		.call_json(
			"PutMetricData",
			&value!({ "Namespace": namespace, "MetricData": metrics }),
		)
		.await?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::Arc;
	use std::sync::Mutex;

	/// The datum's wire shape is what an alarm's `dimensions` must match
	/// exactly: a metric published under a different dimension name is a
	/// metric no alarm ever reads, and an alarm on a metric that never appears
	/// sits in `INSUFFICIENT_DATA` looking exactly like a healthy one.
	#[beet_core::test]
	fn a_datum_carries_the_dimension_its_alarm_names() {
		MetricDatum::rate("BounceRate", 0.012)
			.to_json("Domain", "news.example.com")
			.to_json_string()
			.as_str()
			.xpect_eq(
				r#"{"MetricName":"BounceRate","Value":0.012,"Unit":"None","Dimensions":[{"Name":"Domain","Value":"news.example.com"}]}"#,
			);
	}

	/// The batch is one json `PutMetricData` naming the namespace, every datum
	/// under the one dimension; an empty batch calls nothing.
	#[beet_core::test]
	async fn publishes_the_batch_as_one_call() {
		let calls = Arc::new(Mutex::new(Vec::<(String, Value)>::new()));
		let recorded = calls.clone();
		let client = AwsClient::new(AwsService::MONITORING, "ap-southeast-2")
			.with_credentials(AwsCredentials::new("AKID", "secret"))
			.with_transport(move |request| {
				let recorded = recorded.clone();
				Box::pin(async move {
					request.url().to_string().xpect_eq(
						"https://monitoring.ap-southeast-2.amazonaws.com/",
					);
					let target = request
						.headers
						.first_raw("x-amz-target")
						.unwrap_or_default()
						.to_string();
					let body = json_ext::from_slice::<Value>(
						&request.body.into_bytes().await?,
					)?;
					recorded.lock().unwrap().push((target, body));
					// a successful PutMetricData answers an empty body
					Response::ok().xok()
				})
			});
		put_metric_data_with(
			&client,
			"Beet/Mail",
			"Domain",
			"news.example.com",
			&[],
		)
		.await
		.unwrap();
		calls.lock().unwrap().len().xpect_eq(0);
		put_metric_data_with(
			&client,
			"Beet/Mail",
			"Domain",
			"news.example.com",
			&[
				MetricDatum::rate("BounceRate", 0.012),
				MetricDatum::count("Sent", 40.),
			],
		)
		.await
		.unwrap();
		calls.lock().unwrap().clone().xpect_eq(vec![(
			"GraniteServiceVersion20100801.PutMetricData".to_string(),
			value!({
				"Namespace": "Beet/Mail",
				"MetricData": [
					{
						"MetricName": "BounceRate",
						"Value": 0.012,
						"Unit": "None",
						"Dimensions": [{ "Name": "Domain", "Value": "news.example.com" }],
					},
					{
						"MetricName": "Sent",
						"Value": 40.0,
						"Unit": "Count",
						"Dimensions": [{ "Name": "Domain", "Value": "news.example.com" }],
					},
				],
			}),
		)]);
	}
}

use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use serde_json::json;

/// An SNS topic and the access policy that lets services publish into it:
/// where a stack's alarms fire and its event streams land.
///
/// Account-wide by nature, which is why it is a block of its own rather than a
/// field of whatever fires into it. One topic serves every domain a mail stack
/// carries, since an alarm names the domain it fired for and a topic per domain
/// would only move routing into IAM; so the topic is declared ONCE, under the
/// stack that owns it, and everything else names it by its bare
/// [`name`](Self::name). A stack that fires into a topic another stack owns
/// (a restore drill mirroring production's) names it the same way and declares
/// no block, exactly as it names a sibling's SES identity.
///
/// Authored from markup, ie `<SnsTopicBlock label="ses-events"
/// name="beetmash-ses-events" publishers={["ses.amazonaws.com"]}/>`. The name
/// is stated rather than stack-composed because it is what every reference
/// resolves and what every existing subscription is attached to; an empty one
/// composes the ordinary `<app>--<stage>--<label>`.
///
/// ## Adopting a hand-made topic
///
/// `adopt=true` emits an `import` stanza beside the topic and beside its
/// policy, so the first apply takes over a topic that already exists (`2 to
/// import`) rather than failing on the duplicate name. The stanza is a no-op
/// once both are in state, and the flag comes off after that deploy for one
/// reason: an import of a topic a `destroy` has since taken refuses to apply,
/// so a config that still carries it cannot be redeployed from nothing.
///
/// ## What stays hand-made
///
/// Subscriptions. An email subscription's endpoint is a person's address,
/// which belongs in neither a repository nor a plan, and its confirmation is a
/// click in that person's inbox whatever owns the resource. SNS deletes a
/// topic's subscriptions with the topic, so nothing outlives a destroy either
/// way; a fresh account subscribes once, by hand, and confirms once, by hand.
#[derive(
	Debug, Clone, Get, SetWith, Serialize, Deserialize, Component, Reflect,
)]
#[reflect(Component, Default)]
#[component(immutable, on_insert = ErasedBlock::on_insert::<Self>,
	on_remove = ErasedBlock::on_remove
)]
pub struct SnsTopicBlock {
	label: SmolStr,
	/// The topic's name, ie what a consumer's `alarms_topic` or a relay's
	/// `events_topic` resolves. Empty composes `<app>--<stage>--<label>`.
	#[set_with(into)]
	name: SmolStr,
	/// The service principals granted `SNS:Publish`, ie `ses.amazonaws.com`,
	/// each under an `AWS:SourceAccount` condition so only THIS account's use
	/// of the service may publish. The default policy grants nothing to a
	/// service, and a destination SES cannot publish to drops its events
	/// silently.
	publishers: Vec<SmolStr>,
	/// Emit the import stanzas, see the type docs. A one-deploy flag.
	adopt: bool,
}

impl Default for SnsTopicBlock {
	fn default() -> Self { Self::new("") }
}

impl SnsTopicBlock {
	pub fn new(label: impl Into<SmolStr>) -> Self {
		Self {
			label: label.into(),
			name: SmolStr::default(),
			publishers: Vec::new(),
			adopt: false,
		}
	}

	/// Grant `service` publish on this topic.
	pub fn with_publisher(mut self, service: impl Into<SmolStr>) -> Self {
		self.publishers.push(service.into());
		self
	}

	/// The topic's name, declared or composed.
	pub fn topic_name(&self, stack: &ResolvedStack) -> String {
		match self.name.is_empty() {
			true => stack.resource_name(self.label.clone()),
			false => self.name.to_string(),
		}
	}

	/// The terraform ident the topic is emitted under.
	pub fn ident(&self, stack: &ResolvedStack) -> terra::Ident {
		stack.resource_ident(self.label.clone())
	}

	/// An interpolated reference to the topic's arn: what a consumer that can
	/// see this declaration should fire into, since a reference is also the
	/// dependency edge a composed arn lacks. A fresh account otherwise races
	/// the topic against the SES event destination that names it.
	pub fn arn_ref(&self, stack: &ResolvedStack) -> String {
		format!("${{aws_sns_topic.{}.arn}}", self.ident(stack).label())
	}

	/// The arn of a topic named `name` in `stack`'s region, composed against
	/// the account the apply runs in. What a consumer resolves when no block
	/// in its scope declares the topic; needs `data.aws_caller_identity.current`.
	pub fn composed_arn(stack: &ResolvedStack, name: &str) -> Result<String> {
		format!(
			"arn:aws:sns:{}:${{data.aws_caller_identity.current.account_id}}:{name}",
			stack.aws_region()?
		)
		.xok()
	}

	/// The arn the import stanzas name, composed the same way.
	fn import_id(&self, stack: &ResolvedStack) -> Result<String> {
		Self::composed_arn(stack, &self.topic_name(stack))
	}

	/// The access policy: the owner's own statement (what SNS grants by
	/// default, restated so the policy is whole) and one publish statement per
	/// service.
	fn policy(&self, stack: &ResolvedStack, topic_arn: &str) -> String {
		let account = "${data.aws_caller_identity.current.account_id}";
		let mut statements = vec![json!({
			"Sid": "owner",
			"Effect": "Allow",
			"Principal": { "AWS": "*" },
			"Action": [
				"SNS:GetTopicAttributes",
				"SNS:SetTopicAttributes",
				"SNS:AddPermission",
				"SNS:RemovePermission",
				"SNS:DeleteTopic",
				"SNS:Subscribe",
				"SNS:ListSubscriptionsByTopic",
				"SNS:Publish",
			],
			"Resource": topic_arn,
			"Condition": { "StringEquals": { "AWS:SourceOwner": account } },
		})];
		for service in &self.publishers {
			statements.push(json!({
				"Sid": service.split('.').next().unwrap_or_default()
					.xmap(|prefix| format!("{prefix}-publish")),
				"Effect": "Allow",
				"Principal": { "Service": service },
				"Action": "SNS:Publish",
				"Resource": topic_arn,
				"Condition": { "StringEquals": { "AWS:SourceAccount": account } },
			}));
		}
		json!({
			"Version": "2012-10-17",
			"Id": self.topic_name(stack),
			"Statement": statements,
		})
		.to_string()
	}

	/// Rejects a declaration SNS would reject, at config time.
	pub fn validate(&self) -> Result {
		if self.label.is_empty() {
			bevybail!("an SnsTopicBlock needs a label");
		}
		for service in &self.publishers {
			if !service.ends_with(".amazonaws.com") {
				bevybail!(
					"topic '{}' names publisher '{service}', which is not a \
					service principal (ie `ses.amazonaws.com`)",
					self.label
				);
			}
		}
		Ok(())
	}
}

impl Block for SnsTopicBlock {
	fn label(&self) -> &SmolStr { &self.label }
}

impl EmitBlock for SnsTopicBlock {
	fn emit(
		&self,
		stack: &ResolvedStack,
		_deployment: &Deployment,
		config: &mut terra::Config,
	) -> Result {
		self.validate()?;
		// the account the apply runs in: the policy conditions name it, and so
		// does the import id
		config.add_untyped_data_source(
			"aws_caller_identity",
			"current",
			&json!({}),
		)?;
		let topic =
			ResourceDef::new_secondary(self.ident(stack), AwsSnsTopicDetails {
				name: Some(self.topic_name(stack).into()),
				..default()
			});
		let policy = ResourceDef::new_secondary(
			stack.resource_ident(format!("{}--policy", self.label)),
			AwsSnsTopicPolicyDetails {
				arn: topic.field_ref("arn").into(),
				policy: self.policy(stack, &topic.field_ref("arn")).into(),
				..default()
			},
		);
		config.add_resource(&topic)?.add_resource(&policy)?;
		if self.adopt {
			let id = self.import_id(stack)?;
			config
				.add_import(topic.address(), id.clone())?
				.add_import(policy.address(), id)?;
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::Value;

	fn build_config(block: &SnsTopicBlock) -> (ResolvedStack, Value) {
		let (scope, _dir) = RenderScope::test_render_stack(
			(
				Stack::new("beet_infra"),
				AwsRegion::new(aws::region::AP_SOUTHEAST_2),
			),
			|parent| {
				parent.spawn(block.clone());
			},
		);
		let (stack, _deployment, config) = scope.finish().unwrap();
		(stack, config.to_json().into_json())
	}

	/// The declared name is the topic's name and the policy grants exactly the
	/// named services publish, each under the account condition that stops
	/// another account's use of the same service from publishing here.
	#[beet_core::test]
	fn a_named_topic_grants_its_publishers() {
		let (_stack, json) = build_config(
			&SnsTopicBlock::new("ses-events")
				.with_name("beetmash-ses-events")
				.with_publisher("ses.amazonaws.com"),
		);
		let topic =
			&json["resource"]["aws_sns_topic"]["beet_infra__dev__ses_events"];
		topic["name"]
			.as_str()
			.unwrap()
			.xpect_eq("beetmash-ses-events");
		let policy: Value = serde_json::from_str(
			json["resource"]["aws_sns_topic_policy"]
				["beet_infra__dev__ses_events_policy"]["policy"]
				.as_str()
				.unwrap(),
		)
		.unwrap();
		policy["Statement"][1]["Sid"].xpect_eq(json!("ses-publish"));
		policy["Statement"][1]["Principal"]["Service"]
			.xpect_eq(json!("ses.amazonaws.com"));
		policy["Statement"][1]["Condition"]["StringEquals"]
			["AWS:SourceAccount"]
			.xpect_eq(json!("${data.aws_caller_identity.current.account_id}"));
		// and nothing is imported unless asked
		json.get("import").is_none().xpect_true();
	}

	/// An unnamed topic composes like every other resource.
	#[beet_core::test]
	fn an_unnamed_topic_composes_from_the_stack() {
		let (stack, json) = build_config(&SnsTopicBlock::new("alerts"));
		json["resource"]["aws_sns_topic"]["beet_infra__dev__alerts"]["name"]
			.as_str()
			.unwrap()
			.xpect_eq("beet-infra--dev--alerts");
		SnsTopicBlock::new("alerts")
			.arn_ref(&stack)
			.as_str()
			.xpect_eq("${aws_sns_topic.beet_infra__dev__alerts.arn}");
	}

	/// `adopt` imports the topic and its policy by the arn the account
	/// composes, so a hand-made topic joins the stack at the first apply
	/// instead of failing it on the duplicate name.
	#[beet_core::test]
	fn adopt_imports_the_topic_and_its_policy() {
		let (_stack, json) = build_config(
			&SnsTopicBlock::new("ses-events")
				.with_name("beetmash-ses-events")
				.with_adopt(true),
		);
		json["import"].xpect_eq(json!([
			{
				"to": "aws_sns_topic.beet_infra__dev__ses_events",
				"id": "arn:aws:sns:ap-southeast-2:${data.aws_caller_identity.current.account_id}:beetmash-ses-events"
			},
			{
				"to": "aws_sns_topic_policy.beet_infra__dev__ses_events_policy",
				"id": "arn:aws:sns:ap-southeast-2:${data.aws_caller_identity.current.account_id}:beetmash-ses-events"
			}
		]));
	}

	/// A publisher that is not a service principal is a typo, caught before a
	/// plan.
	#[beet_core::test]
	fn a_publisher_must_be_a_service_principal() {
		SnsTopicBlock::new("t")
			.with_publisher("ses")
			.validate()
			.unwrap_err()
			.to_string()
			.xpect_contains("not a service principal");
	}
}

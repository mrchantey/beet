use crate::bindings::*;
use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::json;
use terra::tf_ref;

/// Opinionated terraform configuration for a Lightsail instance:
/// - Key pair for SSH access
/// - Static IP with attachment
/// - Configurable ports
/// - Systemd service user data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightsailStack {
	/// The port the application server listens on.
	pub server_port: u16,
	/// AWS availability zone, defaults to `us-east-1a`.
	pub availability_zone: SmolStr,
	/// Lightsail blueprint ID, defaults to `amazon_linux_2023`.
	pub blueprint_id: SmolStr,
	/// Lightsail bundle ID (instance size), defaults to `nano_3_0`.
	pub bundle_id: SmolStr,
}

impl Default for LightsailStack {
	fn default() -> Self {
		Self {
			server_port: 8080,
			availability_zone: "us-east-1a".into(),
			blueprint_id: "amazon_linux_2023".into(),
			bundle_id: "nano_3_0".into(),
		}
	}
}

impl LightsailStack {
	/// Build a complete [`terra::Config`] for this Lightsail stack.
	pub fn build_config(
		&self,
		cx: &StackContext,
		stack: &Stack,
	) -> terra::Config {
		let keypair_ident = cx.resource_ident("keypair");
		let keypair = terra::ResourceDef::new_secondary(
			keypair_ident.clone(),
			AwsLightsailKeyPairDetails {
				name_prefix: Some(keypair_ident.primary_identifier().into()),
				..default()
			},
		);

		let ip_ident = cx.resource_ident("ip");
		let static_ip = terra::ResourceDef::new_secondary(
			ip_ident.clone(),
			AwsLightsailStaticIpDetails::new(
				ip_ident.primary_identifier().into(),
			),
		);

		let instance_ident = cx.resource_ident("instance");
		let mut instance_details = AwsLightsailInstanceDetails::new(
			self.availability_zone.clone(),
			self.blueprint_id.clone(),
			self.bundle_id.clone(),
			instance_ident.primary_identifier().into(),
		);
		instance_details.key_pair_name = Some(tf_ref(&keypair.field("name")));
		instance_details.user_data = Some(self.build_user_data(cx));
		instance_details.tags = Some(
			[
				(SmolStr::from("Project"), cx.app_name().clone()),
				(SmolStr::from("Stage"), cx.stage().clone()),
			]
			.into_iter()
			.collect(),
		);
		let instance =
			terra::ResourceDef::new_secondary(instance_ident, instance_details);

		let ip_attach = terra::ResourceDef::new_secondary(
			cx.resource_ident("ip_attach"),
			AwsLightsailStaticIpAttachmentDetails::new(
				tf_ref(&instance.field("name")),
				tf_ref(&static_ip.field("name")),
			),
		);

		let mut port_details = AwsLightsailInstancePublicPortsDetails::new(
			tf_ref(&instance.field("name")),
		);
		port_details.port_info = Some(vec![
			AwsLightsailInstancePublicPortsResourceBlockTypePortInfo::new(
				self.server_port as i64,
				"tcp".into(),
				self.server_port as i64,
			),
			AwsLightsailInstancePublicPortsResourceBlockTypePortInfo::new(
				22,
				"tcp".into(),
				22,
			),
		]);
		let ports = terra::ResourceDef::new_secondary(
			cx.resource_ident("ports"),
			port_details,
		);

		terra::Config::default()
			.with_backend(stack.backend())
			.with_resource(&keypair)
			.with_resource(&static_ip)
			.with_resource(&instance)
			.with_resource(&ip_attach)
			.with_resource(&ports)
			.with_output("instance_name", terra::Output {
				value: json!(tf_ref(&instance.field("name")).as_str()),
				description: Some("The Lightsail instance name".into()),
				sensitive: None,
			})
			.with_output("static_ip_address", terra::Output {
				value: json!(tf_ref(&static_ip.field("ip_address")).as_str()),
				description: Some("The static IP address".into()),
				sensitive: None,
			})
	}

	/// Generate a systemd-based user data script for the application.
	fn build_user_data(&self, cx: &StackContext) -> SmolStr {
		let app_name = cx.app_name();
		format!(
			r#"#!/bin/bash
set -euo pipefail
mkdir -p /opt/{app_name}
cat > /etc/systemd/system/{app_name}.service <<'EOF'
[Unit]
Description={app_name}
After=network.target
[Service]
Type=simple
ExecStart=/opt/{app_name}/app
WorkingDirectory=/opt/{app_name}
Restart=always
RestartSec=3
Environment=RUST_LOG=info
[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload
systemctl enable {app_name}.service
"#
		)
		.into()
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test(timeout_ms = 120000)]
	async fn lightsail_config_validates() {
		let cx = StackContext::default();
		let stack = Stack::new(LocalBackend::default());
		let lightsail = LightsailStack::default();
		let config = lightsail.build_config(&cx, &stack);
		config.validate().await.unwrap();
	}
}

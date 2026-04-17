use crate::bindings::*;
use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::json;

/// Opinionated terraform configuration for a Lightsail instance:
/// - Key pair for SSH access
/// - Static IP with attachment
/// - Configurable ports
/// - Systemd service user data
#[derive(Debug, Clone, SetWith, Serialize, Deserialize, Component)]
#[require(ErasedBlock=ErasedBlock::new::<Self>())]
pub struct LightsailBlock {
	/// The port the application server listens on.
	pub server_port: u16,
	/// AWS availability zone, defaults to `us-east-1a`.
	pub availability_zone: SmolStr,
	/// Lightsail blueprint ID, defaults to `amazon_linux_2023`.
	pub blueprint_id: SmolStr,
	/// Lightsail bundle ID (instance size), defaults to `nano_3_0`.
	pub bundle_id: SmolStr,
}

impl Default for LightsailBlock {
	fn default() -> Self {
		Self {
			server_port: 8337,
			availability_zone: "us-east-1a".into(),
			blueprint_id: "amazon_linux_2023".into(),
			bundle_id: "nano_3_0".into(),
		}
	}
}

impl Block for LightsailBlock {
	fn apply_to_config(
		&self,
		stack: &Stack,
		config: &mut terra::Config,
	) -> Result {
		let keypair_ident = stack.resource_ident("keypair");
		let keypair = terra::ResourceDef::new_secondary(
			keypair_ident.clone(),
			AwsLightsailKeyPairDetails {
				name_prefix: Some(keypair_ident.primary_identifier().clone()),
				..default()
			},
		);

		let ip_ident = stack.resource_ident("ip");
		let static_ip = terra::ResourceDef::new_secondary(
			ip_ident.clone(),
			AwsLightsailStaticIpDetails {
				name: ip_ident.primary_identifier().clone(),
				..default()
			},
		);

		let instance_ident = stack.resource_ident("instance");
		let instance_details = AwsLightsailInstanceDetails {
			availability_zone: self.availability_zone.clone(),
			blueprint_id: self.blueprint_id.clone(),
			bundle_id: self.bundle_id.clone(),
			name: instance_ident.primary_identifier().clone(),
			key_pair_name: Some(keypair.field_ref("name").into()),
			user_data: Some(self.build_user_data(stack)),
			tags: Some(
				[
					(SmolStr::from("Project"), stack.app_name().clone()),
					(SmolStr::from("Stage"), stack.stage().clone()),
				]
				.into_iter()
				.collect(),
			),
			..default()
		};
		let instance =
			terra::ResourceDef::new_secondary(instance_ident, instance_details);

		let ip_attach = terra::ResourceDef::new_secondary(
			stack.resource_ident("ip_attach"),
			AwsLightsailStaticIpAttachmentDetails {
				instance_name: instance.field_ref("name").into(),
				static_ip_name: static_ip.field_ref("name").into(),
				..default()
			},
		);

		let port_details = AwsLightsailInstancePublicPortsDetails {
			instance_name: instance.field_ref("name").into(),
			port_info: Some(vec![
				AwsLightsailInstancePublicPortsResourceBlockTypePortInfo {
					from_port: self.server_port as i64,
					protocol: "tcp".into(),
					to_port: self.server_port as i64,
					..default()
				},
				AwsLightsailInstancePublicPortsResourceBlockTypePortInfo {
					from_port: 22,
					protocol: "tcp".into(),
					to_port: 22,
					..default()
				},
			]),
			..default()
		};
		let ports = terra::ResourceDef::new_secondary(
			stack.resource_ident("ports"),
			port_details,
		);

		config
			.add_resource(&keypair)?
			.add_resource(&static_ip)?
			.add_resource(&instance)?
			.add_resource(&ip_attach)?
			.add_resource(&ports)?
			.add_output("instance_name", terra::Output {
				value: json!(instance.field_ref("name")),
				description: Some("The Lightsail instance name".into()),
				sensitive: None,
			})?
			.add_output("static_ip_address", terra::Output {
				value: json!(static_ip.field_ref("ip_address")),
				description: Some("The static IP address".into()),
				sensitive: None,
			})?
			.add_output("ssh_private_key", terra::Output {
				value: json!(keypair.field_ref("private_key")),
				description: Some("SSH private key for the instance".into()),
				sensitive: Some(true),
			})?;

		Ok(())
	}
}

impl LightsailBlock {
	/// Generate a systemd-based user data script for the application.
	fn build_user_data(&self, stack: &Stack) -> SmolStr {
		let app_name = stack.app_name();
		let server_port = self.server_port;
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
Environment=BEET_HOST=0.0.0.0
Environment=BEET_PORT={server_port}
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
	#[ignore = "very slow"]
	async fn validate() {
		let (stack, _dir) = Stack::default_local();
		let block = LightsailBlock::default();
		let mut config = stack.create_config();
		block.apply_to_config(&stack, &mut config).unwrap();
		let project = terra::Project::new(&stack, config);
		project.validate().await.unwrap();
	}
}

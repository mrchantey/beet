use crate::bindings::*;
use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::json;

/// Networking mode for the Lightsail instance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum LightsailNetworking {
	/// Public static IPv4 address attached to the instance.
	#[default]
	StaticIpv4,
	/// IPv6-only networking (no static IPv4).
	Ipv6,
}

/// Opinionated terraform configuration for a Lightsail instance:
/// - Key pair for SSH access
/// - Static IP with attachment (configurable via networking mode)
/// - Configurable ports
/// - Systemd service user data
#[derive(Debug, Clone, Get, SetWith, Serialize, Deserialize, Component)]
#[component(immutable, on_add = ErasedBlock::on_add::<LightsailBlock>)]
pub struct LightsailBlock {
	/// Label used as a prefix for all terraform resources.
	/// Also used as the artifact name.
	label: SmolStr,
	/// The port the application server listens on.
	pub server_port: u16,
	/// AWS availability zone, defaults to `us-east-1a`.
	pub availability_zone: SmolStr,
	/// Lightsail blueprint ID, defaults to `amazon_linux_2023`.
	pub blueprint_id: SmolStr,
	/// Lightsail bundle ID (instance size), defaults to `nano_3_0`.
	pub bundle_id: SmolStr,
	/// Networking mode, defaults to static IPv4.
	pub networking: LightsailNetworking,
}

impl Default for LightsailBlock {
	fn default() -> Self {
		Self {
			label: "main-lightsail".into(),
			server_port: 8337,
			availability_zone: "us-east-1a".into(),
			blueprint_id: "amazon_linux_2023".into(),
			bundle_id: "nano_3_0".into(),
			networking: LightsailNetworking::default(),
		}
	}
}

impl LightsailBlock {
	/// Build a prefixed label for terraform resources.
	pub fn build_label(&self, suffix: &str) -> String {
		format!("{}--{}", self.label, suffix)
	}

	/// Resolve the SSH user based on the blueprint.
	/// Amazon Linux uses `ec2-user`, Ubuntu uses `ubuntu`.
	pub fn ssh_user(&self) -> &str {
		if self.blueprint_id.contains("ubuntu") {
			"ubuntu"
		} else {
			"ec2-user"
		}
	}

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
Environment=BEET_ASSETS_DIR=/opt/{app_name}/assets
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

impl Block for LightsailBlock {
	fn artifact_label(&self) -> Option<&str> { Some(&self.label) }

	fn apply_to_config(
		&self,
		_entity: &EntityRef,
		stack: &Stack,
		config: &mut terra::Config,
	) -> Result {
		// key pair for SSH access
		let keypair_ident =
			stack.resource_ident(self.build_label("keypair"));
		let keypair = terra::ResourceDef::new_secondary(
			keypair_ident.clone(),
			AwsLightsailKeyPairDetails {
				name_prefix: Some(keypair_ident.primary_identifier().clone()),
				..default()
			},
		);

		// instance configuration
		let instance_ident =
			stack.resource_ident(self.build_label("instance"));
		let mut instance_details = AwsLightsailInstanceDetails {
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

		// enable dual-stack for IPv6 networking
		if matches!(self.networking, LightsailNetworking::Ipv6) {
			instance_details.ip_address_type = Some("dualstack".into());
		}

		let instance = terra::ResourceDef::new_secondary(
			instance_ident,
			instance_details,
		);

		// port rules
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
			stack.resource_ident(self.build_label("ports")),
			port_details,
		);

		// always add keypair, instance, and ports
		config
			.add_resource(&keypair)?
			.add_resource(&instance)?
			.add_resource(&ports)?;

		// conditionally add static IP resources and resolve public address
		let (public_address_value, ip_mode) = match &self.networking {
			LightsailNetworking::StaticIpv4 => {
				let ip_ident =
					stack.resource_ident(self.build_label("ip"));
				let static_ip = terra::ResourceDef::new_secondary(
					ip_ident.clone(),
					AwsLightsailStaticIpDetails {
						name: ip_ident.primary_identifier().clone(),
						..default()
					},
				);
				let ip_attach = terra::ResourceDef::new_secondary(
					stack.resource_ident(self.build_label("ip_attach")),
					AwsLightsailStaticIpAttachmentDetails {
						instance_name: instance.field_ref("name").into(),
						static_ip_name: static_ip.field_ref("name").into(),
						..default()
					},
				);
				let addr = json!(static_ip.field_ref("ip_address"));
				config
					.add_resource(&static_ip)?
					.add_resource(&ip_attach)?;
				(addr, "static_ipv4")
			}
			LightsailNetworking::Ipv6 => {
				let addr = json!(instance.field_ref("ipv6_addresses[0]"));
				(addr, "ipv6")
			}
		};

		// outputs
		config
			.add_output("instance_name", terra::Output {
				value: json!(instance.field_ref("name")),
				description: Some("The Lightsail instance name".into()),
				sensitive: None,
			})?
			.add_output("public_address", terra::Output {
				value: public_address_value,
				description: Some("The public address of the instance".into()),
				sensitive: None,
			})?
			.add_output("ssh_private_key", terra::Output {
				value: json!(keypair.field_ref("private_key")),
				description: Some("SSH private key for the instance".into()),
				sensitive: Some(true),
			})?
			.add_output("ssh_user", terra::Output {
				value: json!(self.ssh_user()),
				description: Some("SSH user for the instance".into()),
				sensitive: None,
			})?
			.add_output("ip_mode", terra::Output {
				value: json!(ip_mode),
				description: Some("Networking mode of the instance".into()),
				sensitive: None,
			})?;

		Ok(())
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
		let mut world = World::new();
		block
			.apply_to_config(
				&world.spawn(()).as_readonly(),
				&stack,
				&mut config,
			)
			.unwrap();
		let project = terra::Project::new(&stack, config);
		project.validate().await.unwrap();
	}
}

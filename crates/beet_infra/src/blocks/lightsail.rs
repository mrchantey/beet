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
/// - IAM user with S3 read access for runtime asset retrieval via aws_sdk
/// - Static IP with attachment (configurable via networking mode)
/// - Systemd service that fetches its binary from S3 on startup
/// - Optional HTTPS via Caddy reverse proxy with automatic Let's Encrypt
#[derive(Debug, Clone, Get, SetWith, Serialize, Deserialize, Component)]
#[component(immutable, on_add = ErasedBlock::on_add::<LightsailBlock>)]
pub struct LightsailBlock {
	/// Label used as a prefix for all terraform resources.
	/// Also used as the artifact name.
	label: SmolStr,
	/// Tofu variables to be inserted as environment variables
	/// in the lightsail instance.
	#[serde(default)]
	env_vars: Vec<Variable>,
	/// Optional domain for HTTPS via Caddy reverse proxy with automatic
	/// Let's Encrypt certificates. When `None`, serves plain HTTP on port 80.
	/// DNS must be configured to point this domain to the instance's public IP.
	#[set_with(unwrap_option, into)]
	domain: Option<SmolStr>,
	/// AWS availability zone. Defaults to the stack's region with suffix 'a', ie `us-west-2a`.
	#[set_with(unwrap_option, into)]
	availability_zone: Option<SmolStr>,
	/// Lightsail blueprint ID, defaults to `amazon_linux_2023`.
	blueprint_id: SmolStr,
	/// Lightsail bundle ID (instance size), defaults to `nano_3_0`.
	bundle_id: SmolStr,
	/// Networking mode, defaults to static IPv4.
	networking: LightsailNetworking,
}

impl Default for LightsailBlock {
	fn default() -> Self {
		Self {
			label: "main-lightsail".into(),
			domain: None,
			availability_zone: None,
			blueprint_id: "amazon_linux_2023".into(),
			bundle_id: "nano_3_0".into(),
			networking: LightsailNetworking::default(),
			env_vars: Vec::new(),
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

	/// The port the application server listens on.
	/// When a domain is set (HTTPS mode), the app runs behind Caddy on an
	/// internal port. Otherwise the app binds directly to port 80.
	fn app_port(&self) -> u16 {
		if self.domain.is_some() {
			beet_net::prelude::DEFAULT_SERVER_PORT
		} else {
			80
		}
	}

	/// Build the user data script that provisions the instance.
	/// Downloads the binary from S3 and creates a systemd service with
	/// AWS credentials so the binary can access S3 at runtime via aws_sdk.
	///
	/// The `access_key_id_ref` and `access_key_secret_ref` are terraform
	/// interpolation expressions (ie `${aws_iam_access_key.xxx.id}`) that
	/// get resolved by terraform before the script runs on the instance.
	fn build_user_data(
		&self,
		stack: &Stack,
		access_key_id_ref: &str,
		access_key_secret_ref: &str,
	) -> SmolStr {
		let app_name = stack.app_name();
		let region = stack.aws_region();
		let bucket = stack.artifact_bucket_name();
		let deploy_id = stack.deploy_id();
		let deploy_timestamp = stack.deploy_timestamp();
		let label = &self.label;
		let app_port = self.app_port();

		// build optional HTTPS setup via Caddy
		let https_setup = if let Some(domain) = &self.domain {
			let caddyfile =
				format!("{domain} {{\n    reverse_proxy localhost:8337\n}}");
			format!(
				r#"
# install Caddy for HTTPS reverse proxy with automatic Let's Encrypt
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/setup.rpm.sh' | bash
dnf install -y caddy

cat > /etc/caddy/Caddyfile <<'CADDY_EOF'
{caddyfile}
CADDY_EOF

systemctl enable --now caddy
"#
			)
		} else {
			String::new()
		};

		// build CloudWatch agent setup for log forwarding
		let stage = stack.stage();
		let cloudwatch_setup = format!(
			r#"
# install and configure CloudWatch agent for log forwarding
dnf install -y amazon-cloudwatch-agent
cat > /opt/aws/amazon-cloudwatch-agent/etc/common-config.toml <<'CCEOF'
[credentials]
shared_credential_profile = "default"
shared_credential_file = "/root/.aws/credentials"
CCEOF
cat > /opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json <<'CWEOF'
{{
  "agent": {{
    "run_as_user": "root",
    "region": "{region}"
  }},
  "logs": {{
    "logs_collected": {{
      "files": {{
        "collect_list": [
          {{
            "file_path": "/var/log/{app_name}.log",
            "log_group_name": "/{app_name}/{stage}",
            "log_stream_name": "{app_name}",
            "retention_in_days": 30
          }}
        ]
      }}
    }}
  }}
}}
CWEOF
/opt/aws/amazon-cloudwatch-agent/bin/amazon-cloudwatch-agent-ctl -a fetch-config -m onPremise -s -c file:/opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json
"#
		);

		// uses __PLACEHOLDER__ tokens for terraform refs that contain ${}
		// which would conflict with Rust's format! macro
		let script = format!(
			r#"#!/bin/bash
set -euo pipefail

# configure AWS credentials for binary download and runtime S3 access
mkdir -p /root/.aws
cat > /root/.aws/credentials <<CREDS
[default]
aws_access_key_id = __ACCESS_KEY_ID__
aws_secret_access_key = __ACCESS_KEY_SECRET__
CREDS
cat > /root/.aws/config <<CONF
[default]
region = {region}
CONF

# download application binary from S3
mkdir -p /opt/{app_name}
aws s3 cp "s3://{bucket}/versions/{deploy_id}/{label}" /opt/{app_name}/app
chmod +x /opt/{app_name}/app

# create systemd service with AWS credentials for runtime S3 access
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
StandardOutput=append:/var/log/{app_name}.log
StandardError=append:/var/log/{app_name}.log
Environment=RUST_LOG=info
Environment=BEET_HOST=0.0.0.0
Environment=BEET_PORT={app_port}
Environment=BEET_DEPLOY_ID={deploy_id}
Environment=BEET_DEPLOY_TIMESTAMP={deploy_timestamp}
Environment=AWS_REGION={region}
Environment=AWS_ACCESS_KEY_ID=__ACCESS_KEY_ID__
Environment=AWS_SECRET_ACCESS_KEY=__ACCESS_KEY_SECRET__
__ENV_VARS__
[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload
systemctl enable --now {app_name}.service
{https_setup}{cloudwatch_setup}"#
		);

		// build env var lines for terraform variable references
		let env_var_lines: String = self
			.env_vars
			.iter()
			.map(|variable| {
				format!(
					"Environment={}=__VAR_{}__",
					variable.key(),
					variable.key()
				)
			})
			.collect::<Vec<_>>()
			.join("\n");

		// replace placeholder tokens with terraform interpolation expressions
		let mut script = script
			.replace("__ACCESS_KEY_ID__", access_key_id_ref)
			.replace("__ACCESS_KEY_SECRET__", access_key_secret_ref)
			.replace("__ENV_VARS__", &env_var_lines);

		// replace env_var placeholders with terraform variable references
		for variable in &self.env_vars {
			script = script.replace(
				&format!("__VAR_{}__", variable.key()),
				&variable.tf_var_ref(),
			);
		}

		script.into()
	}
}

impl Block for LightsailBlock {
	fn artifact_label(&self) -> Option<&str> { Some(&self.label) }
	fn variables(&self) -> &[Variable] { &self.env_vars }

	fn apply_to_config(
		&self,
		_entity: &EntityRef,
		stack: &Stack,
		config: &mut terra::Config,
	) -> Result {
		// IAM user for S3 access (binary download + runtime asset retrieval)
		let user_ident = stack.resource_ident(self.build_label("deploy-user"));
		let user = terra::ResourceDef::new_primary(
			user_ident.clone(),
			AwsIamUserDetails {
				name: user_ident.primary_identifier().clone(),
				..default()
			},
		);
		let user_name_ref = user.field_ref("name");

		// grant the user S3 read access for artifacts and assets
		let policy_ident =
			stack.resource_ident(self.build_label("deploy-s3-policy"));
		let policy = terra::ResourceDef::new_secondary(
			policy_ident,
			AwsIamUserPolicyAttachmentDetails {
				user: user_name_ref.clone().into(),
				policy_arn: "arn:aws:iam::aws:policy/AmazonS3ReadOnlyAccess"
					.into(),
				..default()
			},
		);

		// grant the user CloudWatch Logs write access for the CloudWatch agent
		let cw_policy_ident =
			stack.resource_ident(self.build_label("deploy-cw-policy"));
		let cw_policy = terra::ResourceDef::new_secondary(
			cw_policy_ident,
			AwsIamUserPolicyAttachmentDetails {
				user: user_name_ref.clone().into(),
				policy_arn: "arn:aws:iam::aws:policy/CloudWatchAgentServerPolicy"
					.into(),
				..default()
			},
		);

		// access key for the user
		let key_ident = stack.resource_ident(self.build_label("deploy-key"));
		let access_key = terra::ResourceDef::new_secondary(
			key_ident.clone(),
			AwsIamAccessKeyDetails {
				user: user_name_ref.clone().into(),
				..default()
			},
		);
		let access_key_id_ref = access_key.field_ref("id");
		let access_key_secret_ref = access_key.field_ref("secret");

		// key pair for SSH access
		let keypair_ident = stack.resource_ident(self.build_label("keypair"));
		let keypair = terra::ResourceDef::new_secondary(
			keypair_ident.clone(),
			AwsLightsailKeyPairDetails {
				name_prefix: Some(keypair_ident.primary_identifier().clone()),
				..default()
			},
		);

		// declare terraform variables for env_vars
		for variable in &self.env_vars {
			config.ensure_variable(
				variable.key().as_str(),
				variable.tf_declaration(),
			);
		}

		// instance with self-provisioning user data
		let instance_ident = stack.resource_ident(self.build_label("instance"));
		let user_data = self.build_user_data(
			stack,
			&access_key_id_ref,
			&access_key_secret_ref,
		);
		let mut instance_details = AwsLightsailInstanceDetails {
			availability_zone: self
				.availability_zone
				.clone()
				.unwrap_or_else(|| format!("{}a", stack.aws_region()).into()),
			blueprint_id: self.blueprint_id.clone(),
			bundle_id: self.bundle_id.clone(),
			name: instance_ident.primary_identifier().clone(),
			key_pair_name: Some(keypair.field_ref("name").into()),
			user_data: Some(user_data),
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

		let instance =
			terra::ResourceDef::new_secondary(instance_ident, instance_details);

		// port rules: HTTP (80), SSH (22), and optionally HTTPS (443)
		let mut port_info = vec![
			AwsLightsailInstancePublicPortsResourceBlockTypePortInfo {
				from_port: 80,
				protocol: "tcp".into(),
				to_port: 80,
				..default()
			},
			AwsLightsailInstancePublicPortsResourceBlockTypePortInfo {
				from_port: 22,
				protocol: "tcp".into(),
				to_port: 22,
				..default()
			},
		];
		if self.domain.is_some() {
			port_info.push(
				AwsLightsailInstancePublicPortsResourceBlockTypePortInfo {
					from_port: 443,
					protocol: "tcp".into(),
					to_port: 443,
					..default()
				},
			);
		}
		let ports = terra::ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("ports")),
			AwsLightsailInstancePublicPortsDetails {
				instance_name: instance.field_ref("name").into(),
				port_info: Some(port_info),
				..default()
			},
		);

		// add core resources
		config
			.add_resource(&user)?
			.add_resource(&policy)?
			.add_resource(&cw_policy)?
			.add_resource(&access_key)?
			.add_resource(&keypair)?
			.add_resource(&instance)?
			.add_resource(&ports)?;

		// conditionally add static IP resources and resolve public address
		let (public_address_value, ip_mode) = match &self.networking {
			LightsailNetworking::StaticIpv4 => {
				let ip_ident = stack.resource_ident(self.build_label("ip"));
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
				config.add_resource(&static_ip)?.add_resource(&ip_attach)?;
				// re-attach static IP when instance is replaced
				config.set_lifecycle(
					"aws_lightsail_static_ip_attachment",
					ip_attach.ident().label(),
					json!({
						"replace_triggered_by": [instance.field("id")]
					}),
				)?;
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

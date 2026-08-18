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
/// - IAM user with S3 + DynamoDB full access for runtime persistence via aws_sdk
/// - Static IP with attachment (configurable via networking mode)
/// - Systemd service that fetches its binary from S3 on startup
/// - Optional HTTPS via Caddy reverse proxy with automatic Let's Encrypt
/// - Optional DNS records pointing each authority at the public address
/// - Optional beet ssh on 22, relocating the management sshd
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
	/// Env vars written directly into the unit's `Environment=` lines, last so
	/// a deploy can override any default, and kept off the `ExecStart` argv.
	/// Not a stronger channel than [`env_vars`](Self::env_vars): the rendered
	/// user data still lands in terraform state either way.
	#[serde(default)]
	#[set_with(skip)]
	secret_env: Vec<(SmolStr, SmolStr)>,
	/// DNS records published at the instance's public address: an `A` record
	/// per provider (`AAAA` under [`Ipv6`](LightsailNetworking::Ipv6)
	/// networking). Each authority also lands in the Caddyfile, so declaring
	/// any records turns on HTTPS for them (see [`domain`](Self::with_domain)).
	#[get(skip)]
	#[serde(default)]
	#[set_with(skip)]
	dns: Vec<DnsProvider>,
	/// Serve the beet ssh TUI on port 22: the runtime config gets
	/// `ssh_port=22`, cloud-init moves the box's own sshd to
	/// [`management_ssh_port`](Self::management_ssh_port) before the unit
	/// starts, and the firewall opens both. The unit runs as root (no `User=`
	/// line), so binding 22 needs no extra capability. Note the Lightsail
	/// browser console only dials 22, so management becomes your own client
	/// plus the key pair on the new port.
	#[serde(default)]
	allow_ssh: bool,
	/// Where the management sshd listens once [`allow_ssh`](Self::allow_ssh)
	/// frees port 22 for the TUI.
	management_ssh_port: u16,
	/// The deployed binary's [`BootstrapConfig`], split at render time by
	/// [`BootstrapConfig::split_channels`]: boot selection (the store, the
	/// `--server` set) rides the systemd `ExecStart` invocation, ambient service
	/// config rides the unit's `Environment=` lines. The platform bindings the
	/// block owns (the bind host, the app port, the deploy identity) are merged in
	/// at render time.
	#[serde(default)]
	bootstrap: BootstrapConfig,
	/// Optional manually-routed domain for HTTPS via Caddy reverse proxy with
	/// automatic Let's Encrypt certificates: external DNS must point it at the
	/// instance's public IP. For records this block manages, declare
	/// [`dns`](Self::with_dns) providers instead; Caddy serves the union. With
	/// neither, the app's own port is opened for plain HTTP.
	#[set_with(unwrap_option, into)]
	domain: Option<SmolStr>,
	/// AWS availability zone. Defaults to the stack's region with suffix 'a', ie `us-west-2a`.
	#[set_with(unwrap_option, into)]
	availability_zone: Option<SmolStr>,
	/// Lightsail blueprint ID, defaults to `amazon_linux_2023`.
	blueprint_id: SmolStr,
	/// Lightsail bundle ID (instance size), defaults to `nano_3_0`.
	#[set_with(into)]
	bundle_id: SmolStr,
	/// Networking mode, defaults to static IPv4.
	networking: LightsailNetworking,
	/// Explicit port the app server listens on. When `None`, resolved from
	/// `--port` / `BEET_HTTP_PORT` or
	/// [`DEFAULT_HTTP_PORT`](beet_net::prelude::DEFAULT_HTTP_PORT) via
	/// [`app_port`](Self::app_port). Until infra declaration is wired to the
	/// served site's state (like SST), this must match the markup `HttpServer{port}`.
	#[get(skip)]
	#[set_with(unwrap_option)]
	app_port: Option<u16>,
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
			secret_env: Vec::new(),
			dns: Vec::new(),
			allow_ssh: false,
			management_ssh_port: 2222,
			bootstrap: default(),
			app_port: None,
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

	/// Add a DNS record published at the instance's public address
	/// (repeatable, see [`dns`](Self::dns)).
	pub fn with_dns(mut self, dns: DnsProvider) -> Self {
		self.dns.push(dns);
		self
	}

	/// Add an env var delivered off-argv (repeatable, see
	/// [`secret_env`](Self::secret_env)).
	pub fn with_secret_env(
		mut self,
		key: impl Into<SmolStr>,
		value: impl Into<SmolStr>,
	) -> Self {
		self.secret_env.push((key.into(), value.into()));
		self
	}

	/// Every hostname Caddy serves: the manual [`domain`](Self::with_domain)
	/// plus each [`dns`](Self::with_dns) authority. Non-empty means Caddy
	/// terminates TLS on 80/443 and the app port stays private.
	fn caddy_hostnames(&self) -> Vec<&SmolStr> {
		self.domain
			.iter()
			.chain(self.dns.iter().map(|dns| dns.authority()))
			.collect()
	}

	/// Publish each [`dns`](Self::with_dns) record at `address_ref`, a terra
	/// field-ref resolving to the instance's public IP.
	fn emit_dns(
		&self,
		stack: &Stack,
		config: &mut terra::Config,
		address_ref: &str,
		ipv6: bool,
	) -> Result {
		for dns in &self.dns {
			let suffix = dns.authority().replace('.', "-");
			dns.emit_address(
				stack,
				config,
				&self.build_label(&format!("dns-{suffix}")),
				address_ref,
				ipv6,
			)?;
		}
		Ok(())
	}

	/// The port the application server listens on: the block's explicit
	/// [`app_port`](Self::with_app_port) if set, else `--port` /
	/// `BEET_HTTP_PORT`, else
	/// [`DEFAULT_HTTP_PORT`](beet_net::prelude::DEFAULT_HTTP_PORT) (8337). Must
	/// match the served site's markup port. With a domain Caddy reverse-proxies
	/// 443 -> this port; without one the instance opens this port publicly.
	fn app_port(&self) -> u16 {
		beet_net::prelude::resolve_server_port(self.app_port)
	}

	/// The CloudWatch log group the instance forwards its app logs to, the
	/// single source of truth shared by the cloud-init agent config and
	/// [`AwsWatch::for_lightsail`](crate::prelude::AwsWatch::for_lightsail).
	/// Includes the label so distinct blocks in one stack do not collide.
	pub fn log_group(&self, stack: &Stack) -> String {
		format!("/{}/{}/{}", stack.app_name(), self.label, stack.stage())
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
	) -> Result<SmolStr> {
		let app_name = stack.app_name();
		let region = stack.aws_region();
		let bucket = stack.artifact_bucket_name();
		let deploy_id = stack.deploy_id();
		let deploy_timestamp = stack.deploy_timestamp();
		let label = &self.label;
		let app_port = self.app_port();
		// the deployed binary's config, with the platform bindings this block owns
		// merged in, then split onto its two channels.
		let runtime = BootstrapConfig {
			host: Some(core::net::Ipv4Addr::UNSPECIFIED.into()),
			http_port: Some(app_port),
			ssh_port: self.allow_ssh.then_some(22),
			// the deployed stage, so the running process reports (and names cloud
			// resources for) the stage it is actually deployed to.
			stage: stack.stage().clone(),
			deploy_id: Some(deploy_id.to_string().into()),
			deploy_timestamp: Some(deploy_timestamp.to_string().into()),
			..self.bootstrap.clone()
		};
		let (argv, env) = runtime.split_channels();
		// boot selection rides the `ExecStart` invocation. `to_argv` validates every
		// token against a shell-safe charset, so an arg that would corrupt the unit
		// file is a render error rather than a silently broken deploy.
		let exec_args = argv
			.to_argv()?
			.iter()
			.map(|arg| format!(" {arg}"))
			.collect::<String>();
		// ambient service config rides `Environment=` lines, named by the same
		// table the runtime parses.
		let bootstrap_env = env
			.to_env()
			.iter()
			.map(|(key, value)| format!("Environment={key}={value}\n"))
			.collect::<String>();
		// secrets last, so a deploy can override any default above.
		let secret_env_lines = self
			.secret_env
			.iter()
			.map(|(key, value)| format!("Environment={key}={value}\n"))
			.collect::<String>();

		// free port 22 for the TUI: comment out any explicit `Port` in the main
		// sshd config and declare the management port in a drop-in, ordered
		// before the unit starts so 22 is unbound when the app claims it
		let ssh_setup = if self.allow_ssh {
			let management_ssh_port = self.management_ssh_port;
			format!(
				r#"
# move the management sshd off 22 so the app can serve ssh there
sed -i 's/^Port /#Port /' /etc/ssh/sshd_config
mkdir -p /etc/ssh/sshd_config.d
echo 'Port {management_ssh_port}' > /etc/ssh/sshd_config.d/90-beet-management.conf
systemctl restart sshd
"#
			)
		} else {
			String::new()
		};

		// build optional HTTPS setup via Caddy, one site block serving every
		// hostname (the manual domain + the dns authorities)
		let caddy_hostnames = self.caddy_hostnames();
		let https_setup = if caddy_hostnames.is_empty() {
			String::new()
		} else {
			let hostnames = caddy_hostnames
				.iter()
				.map(|hostname| hostname.as_str())
				.collect::<Vec<_>>()
				.join(", ");
			let caddyfile = format!(
				"{hostnames} {{\n    reverse_proxy localhost:{app_port}\n}}"
			);
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
		};

		// build CloudWatch agent setup for log forwarding; the log group matches
		// `AwsWatch::for_lightsail` so `watch` tails the same group
		let log_group = self.log_group(stack);
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
            "log_group_name": "{log_group}",
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
{ssh_setup}
# create systemd service with AWS credentials for runtime S3 access
cat > /etc/systemd/system/{app_name}.service <<'EOF'
[Unit]
Description={app_name}
After=network.target
[Service]
Type=simple
ExecStart=/opt/{app_name}/app{exec_args}
WorkingDirectory=/opt/{app_name}
Restart=always
RestartSec=3
StandardOutput=append:/var/log/{app_name}.log
StandardError=append:/var/log/{app_name}.log
Environment=RUST_LOG=info
Environment=AWS_REGION={region}
Environment=AWS_ACCESS_KEY_ID=__ACCESS_KEY_ID__
Environment=AWS_SECRET_ACCESS_KEY=__ACCESS_KEY_SECRET__
{bootstrap_env}__ENV_VARS__{secret_env_lines}[Install]
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
					"Environment={}=__VAR_{}__\n",
					variable.key(),
					variable.key()
				)
			})
			.collect();

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

		SmolStr::from(script).xok()
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

		// grant the user S3 full access for artifacts, assets, and runtime persistence
		let policy_ident =
			stack.resource_ident(self.build_label("deploy-s3-policy"));
		let policy = terra::ResourceDef::new_secondary(
			policy_ident,
			AwsIamUserPolicyAttachmentDetails {
				user: user_name_ref.clone().into(),
				policy_arn: "arn:aws:iam::aws:policy/AmazonS3FullAccess".into(),
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
				policy_arn:
					"arn:aws:iam::aws:policy/CloudWatchAgentServerPolicy".into(),
				..default()
			},
		);

		// grant the user DynamoDB access for runtime table stores, eg analytics
		let dynamo_policy_ident =
			stack.resource_ident(self.build_label("deploy-dynamo-policy"));
		let dynamo_policy = terra::ResourceDef::new_secondary(
			dynamo_policy_ident,
			AwsIamUserPolicyAttachmentDetails {
				user: user_name_ref.clone().into(),
				policy_arn: "arn:aws:iam::aws:policy/AmazonDynamoDBFullAccess"
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

		// log group the instance's CloudWatch agent forwards to; declared here so
		// `tofu destroy` removes it (the agent reuses the existing group rather
		// than auto-creating an unmanaged one that would leak on teardown)
		let log_group_ident = stack.resource_ident(self.build_label("logs"));
		let log_group = terra::ResourceDef::new_secondary(
			log_group_ident,
			AwsCloudwatchLogGroupDetails {
				name: Some(self.log_group(stack).into()),
				retention_in_days: Some(30),
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
		)?;
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

		// tcp port helper for the firewall entries below
		let tcp_port =
			|port: u16| AwsLightsailInstancePublicPortsResourceBlockTypePortInfo {
				from_port: port.into(),
				protocol: "tcp".into(),
				to_port: port.into(),
				..default()
			};
		// Port 22 is always open: the management sshd by default, the beet ssh
		// TUI under `allow_ssh` (management moves to `management_ssh_port`).
		// With Caddy hostnames TLS terminates on 80/443 and proxies to the app's
		// internal port; without any the app's own port (`app_port`) is opened
		// directly so the binary is publicly reachable.
		let mut port_info = vec![tcp_port(22)];
		if self.allow_ssh {
			port_info.push(tcp_port(self.management_ssh_port));
		}
		if self.caddy_hostnames().is_empty() {
			port_info.push(tcp_port(self.app_port()));
		} else {
			port_info.extend([tcp_port(80), tcp_port(443)]);
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
			.add_resource(&dynamo_policy)?
			.add_resource(&access_key)?
			.add_resource(&keypair)?
			.add_resource(&log_group)?
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
				self.emit_dns(stack, config, &static_ip.field_ref("ip_address"), false)?;
				(addr, "static_ipv4")
			}
			LightsailNetworking::Ipv6 => {
				let addr_ref = instance.field_ref("ipv6_addresses[0]");
				self.emit_dns(stack, config, &addr_ref, true)?;
				(json!(addr_ref), "ipv6")
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

	/// The rendered cloud-init user-data script for a block, ie the systemd unit
	/// the instance provisions itself with.
	fn build_user_data(block: &LightsailBlock) -> (String, TestWorkDir) {
		let (stack, dir) = Stack::default_local();
		let script = block
			.build_user_data(&stack, "${key_id}", "${key_secret}")
			.unwrap();
		(script.to_string(), dir)
	}

	/// The rendered terraform config json for a block.
	fn build_json(block: &LightsailBlock) -> String {
		let (stack, _dir) = Stack::default_local();
		let mut config = stack.create_config();
		let mut world = World::new();
		block
			.apply_to_config(&world.spawn(()).as_readonly(), &stack, &mut config)
			.unwrap();
		config.to_json().to_string()
	}

	/// `allow_ssh`: the TUI takes 22 (`BEET_SSH_PORT=22`), cloud-init relocates
	/// the management sshd before the unit starts, and the firewall opens the
	/// management port. Without it the sshd keeps 22 untouched.
	#[beet_core::test]
	fn allow_ssh_takes_22_and_relocates_management() {
		let block = LightsailBlock::default().with_allow_ssh(true);
		let (script, _dir) = build_user_data(&block);
		script
			.as_str()
			.xpect_contains("Environment=BEET_SSH_PORT=22")
			.xpect_contains("Port 2222")
			.xpect_contains("systemctl restart sshd");
		// the relocation runs before the unit claims 22
		let sshd_move = script.find("systemctl restart sshd").unwrap();
		let unit_start = script.find("systemctl enable --now").unwrap();
		(sshd_move < unit_start).xpect_true();
		build_json(&block).xpect_contains("2222");
		let (script, _dir) = build_user_data(&LightsailBlock::default());
		script.as_str().xnot().xpect_contains("sshd");
	}

	/// Secret env rides the unit's `Environment=` lines, never `ExecStart`.
	#[beet_core::test]
	fn secret_env_rides_environment_lines() {
		let (script, _dir) =
			build_user_data(&LightsailBlock::default().with_secret_env(
				"BEET_SSH_HOST_KEY",
				"abc123",
			));
		script
			.as_str()
			.xpect_contains("Environment=BEET_SSH_HOST_KEY=abc123")
			.xnot()
			.xpect_contains("app abc123");
	}

	/// The IAM user carries the runtime store policies: S3 (site), CloudWatch
	/// (log forwarding) and DynamoDB (analytics).
	#[beet_core::test]
	fn grants_runtime_policies() {
		build_json(&LightsailBlock::default())
			.xpect_contains("AmazonS3FullAccess")
			.xpect_contains("CloudWatchAgentServerPolicy")
			.xpect_contains("AmazonDynamoDBFullAccess");
	}

	/// Declared dns providers emit `A` records at the static IP (proxied flag
	/// preserved), land in the Caddyfile as one site block, and switch the
	/// firewall to 80/443 in place of the app port.
	#[cfg(feature = "cloudflare_dns")]
	#[beet_core::test]
	fn dns_emits_a_records_and_caddy_hostnames() {
		let block = LightsailBlock::default()
			.with_dns(
				DnsProvider::cloudflare("example.org", "zone123")
					.with_proxied(true),
			)
			.with_dns(DnsProvider::cloudflare("app.example.org", "zone123"));
		build_json(&block)
			.xpect_contains("cloudflare_dns_record")
			.xpect_contains("\"type\":\"A\"")
			.xpect_contains("\"proxied\":true")
			.xpect_contains("\"proxied\":false")
			.xpect_contains("\"from_port\":443")
			.xnot()
			.xpect_contains("\"from_port\":8337");
		let (script, _dir) = build_user_data(&block);
		script
			.as_str()
			.xpect_contains("example.org, app.example.org {");
	}

	/// REGRESSION: the unit must write `BEET_HTTP_PORT`, the name the runtime
	/// actually reads. It used to write `BEET_PORT`, which nothing read, so a
	/// deploy on a non-default port silently bound 8337 anyway. Rendering through
	/// `BootstrapConfig` makes that drift unrepresentable.
	#[beet_core::test]
	fn writes_the_port_name_the_runtime_reads() {
		let (script, _dir) =
			build_user_data(&LightsailBlock::default().with_app_port(9001));
		script
			.as_str()
			.xpect_contains("Environment=BEET_HTTP_PORT=9001")
			.xpect_contains("Environment=BEET_HOST=0.0.0.0")
			.xnot()
			.xpect_contains("BEET_PORT=");
	}

	/// Boot selection rides `ExecStart` as validated argv; ambient service config
	/// rides `Environment=` lines. The deployed stage is a platform binding: it
	/// flows from the stack, not the authored bootstrap.
	#[beet_core::test]
	fn splits_exec_and_env_channels() {
		let (stack, _dir) = Stack::default_local();
		let stack = stack.with_stage("staging");
		let script = LightsailBlock::default()
			.with_bootstrap(BootstrapConfig {
				store: Some(StoreUri::parse("s3://beet--dev--app").unwrap()),
				server: Some(ServerFilter::new("http")),
				..default()
			})
			.build_user_data(&stack, "${key_id}", "${key_secret}")
			.unwrap();
		script
			.as_str()
			.xpect_contains(
				"ExecStart=/opt/beet_infra/app --store=s3://beet--dev--app \
				--server=http",
			)
			.xpect_contains("Environment=BEET_STAGE=staging")
			// the store never leaks onto the env channel
			.xnot()
			.xpect_contains("BEET_STORE");
	}

	/// A token that cannot be rendered into a systemd `ExecStart` is a loud error
	/// rather than a corrupted unit file: the old space-join caveat, enforced.
	#[beet_core::test]
	fn rejects_unencodable_exec_args() {
		let (stack, _dir) = Stack::default_local();
		LightsailBlock::default()
			.with_bootstrap(BootstrapConfig {
				path: Some("/my page".into()),
				..default()
			})
			.build_user_data(&stack, "id", "secret")
			.unwrap_err()
			.to_string()
			.xpect_contains("cannot be rendered");
	}

	// drives the native tofu Project, so it cannot compile for wasm
	#[cfg(not(target_arch = "wasm32"))]
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

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
/// - IAM user with a least-privilege inline policy for runtime persistence
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
	/// The positional route the unit dispatches on boot, appended after the
	/// argv channel: `exec /opt/<app>/app --store=.. --server=.. serve`.
	///
	/// An entry whose root is a `CliServer` dispatcher selects its command with
	/// a positional arg, so a deployed site says which of its routes *is* the
	/// site. Distinct from [`BootstrapConfig::path`], which is the opening page
	/// of a tui/ssh surface once booted, not a boot selector.
	#[set_with(unwrap_option, into)]
	exec_route: Option<SmolStr>,
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
			exec_route: None,
		}
	}
}

impl LightsailBlock {
	/// The Caddy release installed as the TLS terminator, pinned so a box built
	/// today and a box built next month are the same box. Bump deliberately.
	pub const CADDY_VERSION: &'static str = "2.11.4";

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

	/// The inline IAM policy document for the box's runtime identity, LOWERED
	/// from the [`AccessGrants`] the stack's blocks declared: a declared bucket
	/// becomes an s3 read, a declared table a dynamodb read/write. The two grants
	/// this block owns internally are added here because nothing declares them:
	/// the artifacts bucket it pulls its binary from at boot, and its own log
	/// group.
	///
	/// Provider-agnostic on the declaring side, provider-specific here: a block
	/// says "this process reads that bucket" and the compute renders whatever its
	/// platform's permission mechanism is (IAM statements here, wrangler bindings
	/// for a Cloudflare compute).
	///
	/// The account segment is a wildcard because the account id is not known at
	/// render time; the resource names are already stage-scoped, and a policy
	/// cannot grant cross-account access anyway (that needs a resource policy on
	/// the far side).
	fn runtime_policy(
		&self,
		stack: &Stack,
		deployment: &Deployment,
		access: &AccessGrants,
	) -> Result<String> {
		let region = stack.region();
		let mut statements = Vec::new();

		// every declared bucket, read-only: the deploy publishes them, the box
		// serves them. Its own artifacts bucket is declared by nothing, so it is
		// added here.
		let mut read_buckets = vec![deployment.artifact_bucket_name(stack)];
		read_buckets
			.extend(access.s3_buckets().iter().map(ToString::to_string));
		statements.push(json!({
			"Sid": "ReadStores",
			"Effect": "Allow",
			"Action": ["s3:GetObject", "s3:ListBucket"],
			"Resource": read_buckets
				.iter()
				.flat_map(|bucket| [
					format!("arn:aws:s3:::{bucket}"),
					format!("arn:aws:s3:::{bucket}/*"),
				])
				.collect::<Vec<_>>()
		}));

		// every declared table, read/write
		for (table, table_region) in access.dynamo_tables() {
			statements.push(json!({
				"Sid": "DeclaredTables",
				"Effect": "Allow",
				"Action": [
					"dynamodb:DescribeTable",
					"dynamodb:GetItem",
					"dynamodb:PutItem",
					"dynamodb:UpdateItem",
					"dynamodb:DeleteItem",
					"dynamodb:Query",
					"dynamodb:Scan",
					"dynamodb:BatchGetItem",
					"dynamodb:BatchWriteItem"
				],
				"Resource": format!("arn:aws:dynamodb:{table_region}:*:table/{table}")
			}));
		}

		// the block's own log group, for the CloudWatch agent
		let log_group = self.log_group(stack);
		statements.push(json!({
			"Sid": "OwnLogGroup",
			"Effect": "Allow",
			"Action": [
				"logs:CreateLogGroup",
				"logs:CreateLogStream",
				"logs:PutLogEvents",
				"logs:DescribeLogStreams"
			],
			"Resource": [
				format!("arn:aws:logs:{region}:*:log-group:{log_group}"),
				format!("arn:aws:logs:{region}:*:log-group:{log_group}:*")
			]
		}));

		json!({ "Version": "2012-10-17", "Statement": statements })
			.to_string()
			.xok()
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
		format!(
			"/{}/{}/{}",
			stack.app_name().unwrap_or_default(),
			self.label,
			stack.stage()
		)
	}

	/// The systemd unit, and the name every path on the box is built from.
	fn service_name(stack: &Stack) -> &str {
		stack.app_name().unwrap_or_default()
	}

	/// Build the user data script that provisions the instance: the Caddy
	/// install, the sshd relocation, the CloudWatch agent and the systemd unit
	/// that runs the app.
	///
	/// ## The rebuild rule
	///
	/// **Everything rendered here is MACHINE config, and any change to it
	/// replaces the instance.** Terraform has no way to update `user_data` in
	/// place, so an edit to this script is a new box: a cold page cache, a
	/// burst-credit balance reset to zero, an outage window and a fresh Let's
	/// Encrypt issuance. That is the right answer for a change to what the box
	/// *is* (a new Caddy, a new blueprint, a new unit definition) and the wrong
	/// answer for a change to what it *runs*.
	///
	/// So APP config is deliberately absent. The versioned artifact key and the
	/// deploy identity live in the artifacts bucket behind
	/// [`ArtifactLedger::release_pointer_key`], and the unit resolves them at
	/// every start through the fetch/run script pair below. A code-only deploy
	/// therefore renders a byte-identical script, terraform plans no change to
	/// the instance, and [`LightsailRelease`] rolls the running unit onto the
	/// new binary instead. Pinned by `code_only_deploy_renders_one_box`.
	///
	/// Adding a value here is a decision to rebuild the box whenever that value
	/// changes. If it changes per deploy it belongs on the release pointer, not
	/// in this script.
	///
	/// The `access_key_id_ref` and `access_key_secret_ref` are terraform
	/// interpolation expressions (ie `${aws_iam_access_key.xxx.id}`) that
	/// get resolved by terraform before the script runs on the instance.
	fn build_user_data(
		&self,
		stack: &Stack,
		deployment: &Deployment,
		access_key_id_ref: &str,
		access_key_secret_ref: &str,
	) -> Result<SmolStr> {
		let app_name = Self::service_name(stack);
		let region = stack.region();
		let app_port = self.app_port();
		// the deployed binary's config, with the platform bindings this block owns
		// merged in, then split onto its two channels. The deploy identity is
		// absent by design: it is per-deploy, so it rides the release pointer.
		let runtime = BootstrapConfig {
			host: Some(core::net::Ipv4Addr::UNSPECIFIED.into()),
			http_port: Some(app_port),
			ssh_port: self.allow_ssh.then_some(22),
			// the deployed stage, so the running process reports (and names cloud
			// resources for) the stage it is actually deployed to.
			stage: stack.stage().into(),
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
			.collect::<String>()
			// the boot route is positional, so it trails every flag.
			.xmap(|args| match &self.exec_route {
				Some(route) => format!("{args} {route}"),
				None => args,
			});
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
			let caddy_version = Self::CADDY_VERSION;
			// the upstream static binary, not a distro package: Caddy publishes no
			// `amzn` rpms, and its repo-setup script still writes an `amzn/2023`
			// baseurl with `skip_if_unavailable=1`, so `dnf install -y caddy`
			// no-opped and the box booted with no TLS terminator at all.
			format!(
				r#"
# install Caddy for HTTPS reverse proxy with automatic Let's Encrypt
curl -sSLf 'https://github.com/caddyserver/caddy/releases/download/v{caddy_version}/caddy_{caddy_version}_linux_amd64.tar.gz' \
  | tar -xz -C /usr/local/bin caddy
chmod +x /usr/local/bin/caddy
# fail the boot loudly rather than serve nothing on 443
/usr/local/bin/caddy version || exit 1

id caddy >/dev/null 2>&1 || useradd --system --home /var/lib/caddy --create-home --shell /usr/sbin/nologin caddy
mkdir -p /etc/caddy /var/lib/caddy
chown -R caddy:caddy /var/lib/caddy

cat > /etc/caddy/Caddyfile <<'CADDY_EOF'
{caddyfile}
CADDY_EOF

cat > /etc/systemd/system/caddy.service <<'CADDY_UNIT_EOF'
[Unit]
Description=Caddy
After=network-online.target
Requires=network-online.target

[Service]
Type=notify
User=caddy
Group=caddy
ExecStart=/usr/local/bin/caddy run --environ --config /etc/caddy/Caddyfile
ExecReload=/usr/local/bin/caddy reload --config /etc/caddy/Caddyfile --force
TimeoutStopSec=5s
LimitNOFILE=1048576
PrivateTmp=true
ProtectSystem=full
AmbientCapabilities=CAP_NET_BIND_SERVICE

[Install]
WantedBy=multi-user.target
CADDY_UNIT_EOF

systemctl daemon-reload
systemctl enable --now caddy
"#
			)
		};

		// build CloudWatch agent setup for log forwarding; the log group matches
		// `AwsWatch::for_lightsail` so `watch` tails the same group.
		//
		// `timestamp_format` is what makes the forwarded event carry the time the
		// line was WRITTEN rather than the time the agent happened to ingest it,
		// so a stall is correlatable against CloudWatch metrics. `timezone` is
		// required alongside it: the agent interprets a parsed stamp as LOCAL by
		// default and beet emits `Z`. `multi_line_start_pattern` reuses that same
		// regex so a panic backtrace stays one event instead of N.
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
            "retention_in_days": 30,
            "timestamp_format": "%Y-%m-%dT%H:%M:%S.%fZ",
            "timezone": "UTC",
            "multi_line_start_pattern": "{{timestamp_format}}"
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

		// the two release scripts, the whole of the machine's knowledge about
		// finding its binary. Neither names a version.
		let fetch_script = self.fetch_script(stack, deployment);
		let run_script = self.run_script(stack, &exec_args);

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

mkdir -p /opt/{app_name} /etc/{app_name}

# install the release scripts: the unit resolves the current binary at every
# start, so this box is described without naming a deploy
cat > /usr/local/bin/{app_name}-fetch <<'FETCH_EOF'
{fetch_script}
FETCH_EOF
cat > /usr/local/bin/{app_name}-run <<'RUN_EOF'
{run_script}
RUN_EOF
chmod +x /usr/local/bin/{app_name}-fetch /usr/local/bin/{app_name}-run
{ssh_setup}
# create systemd service with AWS credentials for runtime S3 access
cat > /etc/systemd/system/{app_name}.service <<'EOF'
[Unit]
Description={app_name}
After=network.target
[Service]
Type=simple
ExecStart=/usr/local/bin/{app_name}-run
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

		// The rendered script is a terraform string, so terraform reads every
		// `${..}` in it as an interpolation. The shell's own parameter
		// expansions (`${BEET_ARTIFACT_KEY:-}`) are not terraform expressions,
		// and one of them is a parse error rather than a mis-render: the colon
		// in `:-` is not valid in an interpolation, so `tofu validate` rejects
		// the whole config. Escape every literal one to `$${..}` BEFORE the
		// placeholders below become real terraform refs, which is exactly why
		// those are placeholders.
		let script = script.replace("${", "$${");

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

	/// The release fetcher installed at `/usr/local/bin/<app>-fetch`: read the
	/// artifacts bucket's release pointer, install the binary it names and the
	/// deploy identity it carries.
	///
	/// A fetch failure is not fatal while a binary is already installed. A
	/// restart after a crash must not be blocked by a transient S3 error, and
	/// the box keeps serving what it has rather than serving nothing.
	fn fetch_script(&self, stack: &Stack, deployment: &Deployment) -> String {
		Self::render_script(
			r#"#!/bin/bash
# Install the release the artifacts bucket currently points at.
set -uo pipefail
mkdir -p /opt/__APP__ /etc/__APP__
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

if ! aws s3 cp "s3://__BUCKET__/__POINTER__" "$tmp/deploy.env" >&2; then
	echo "beet: no release pointer at __POINTER__, keeping the installed binary" >&2
elif ! . "$tmp/deploy.env"; then
	echo "beet: release pointer is unreadable, keeping the installed binary" >&2
elif [ -z "${__ARTIFACT_KEY_VAR__:-}" ]; then
	# guarded: under `set -u` a bare expansion of a missing var would abort the
	# whole script instead of landing here
	echo "beet: release pointer names no __ARTIFACT_KEY_VAR__, keeping the installed binary" >&2
elif ! aws s3 cp "s3://__BUCKET__/$__ARTIFACT_KEY_VAR__" "$tmp/app" >&2; then
	echo "beet: cannot download $__ARTIFACT_KEY_VAR__, keeping the installed binary" >&2
else
	# staged in the destination directory then renamed, so the swap is atomic
	# and never truncates an inode something is still executing
	install -m 755 "$tmp/app" /opt/__APP__/app.next
	install -m 644 "$tmp/deploy.env" /etc/__APP__/deploy.env.next
	mv -f /opt/__APP__/app.next /opt/__APP__/app
	mv -f /etc/__APP__/deploy.env.next /etc/__APP__/deploy.env
	echo "beet: installed release ${BEET_DEPLOY_ID:-unknown}" >&2
fi

if [ ! -x /opt/__APP__/app ]; then
	echo "beet: no binary installed and none could be fetched" >&2
	exit 1
fi
touch /etc/__APP__/deploy.env
"#,
			stack,
			&[
				("__BUCKET__", &deployment.artifact_bucket_name(stack)),
				(
					"__POINTER__",
					&ArtifactLedger::release_pointer_key(&self.label)
						.to_string(),
				),
				("__ARTIFACT_KEY_VAR__", ArtifactLedger::ARTIFACT_KEY_VAR),
			],
		)
	}

	/// The unit's `ExecStart`, at `/usr/local/bin/<app>-run`: install the
	/// current release, load the deploy identity it published, exec it.
	///
	/// Resolving the release per start (rather than at provision time) is the
	/// whole trick: `systemctl restart` is a deploy, and the machine config
	/// stays constant across every one of them. The `exec` keeps the unit's
	/// `MainPID` on the app itself, so a caller can read the running process's
	/// own environment to prove which release is serving.
	fn run_script(&self, stack: &Stack, exec_args: &str) -> String {
		Self::render_script(
			r#"#!/bin/bash
# Launch the current release.
set -euo pipefail
/usr/local/bin/__APP__-fetch
set -a
. /etc/__APP__/deploy.env
set +a
exec /opt/__APP__/app__EXEC_ARGS__
"#,
			stack,
			&[("__EXEC_ARGS__", exec_args)],
		)
	}

	/// The script [`LightsailRelease`] runs on the box: roll the unit onto the
	/// release the artifacts bucket currently points at, and prove it took.
	///
	/// The proof is deliberately the RUNNING process's own environment, read
	/// out of `/proc/<MainPID>/environ`, not the presence of a file or a
	/// successful `systemctl restart`. A binary that downloaded but never
	/// launched, or a unit that restarted into a crash loop, both look like
	/// success from anywhere else.
	///
	/// Idempotent: a box that already serves this release (a freshly rebuilt
	/// one, which pulled it at boot) is left alone rather than bounced.
	///
	/// `poll` is the gap between attempts, and the script gives the unit
	/// `timeout` to appear and `timeout` again to converge, since a replaced
	/// box is still running cloud-init when the deploy arrives.
	pub fn release_script(
		&self,
		stack: &Stack,
		deploy_id: &str,
		timeout: Duration,
		poll: Duration,
	) -> String {
		let poll_secs = poll.as_secs().max(1);
		let attempts = (timeout.as_secs() / poll_secs).max(1);
		Self::render_script(
			r#"#!/bin/bash
# Roll the unit onto a release, and prove the process now running IS it.
set -uo pipefail
unit=__APP__.service
expect=__DEPLOY_ID__

running_release() {
	local pid
	pid=$(systemctl show -p MainPID --value "$unit" 2>/dev/null)
	if [ -z "$pid" ] || [ "$pid" = 0 ]; then return 1; fi
	tr '\0' '\n' < "/proc/$pid/environ" | sed -n 's/^BEET_DEPLOY_ID=//p'
}

# a replaced box is still running cloud-init, so wait for the unit to exist
for _ in $(seq 1 __ATTEMPTS__); do
	systemctl cat "$unit" >/dev/null 2>&1 && break
	sleep __POLL__
done

if [ "$(running_release)" = "$expect" ]; then
	echo "beet: already serving $expect" >&2
else
	echo "beet: restarting $unit onto $expect" >&2
	systemctl restart "$unit" >&2 || true
fi

for _ in $(seq 1 __ATTEMPTS__); do
	if [ "$(systemctl is-active "$unit")" = active ] && [ "$(running_release)" = "$expect" ]; then
		echo "$expect"
		exit 0
	fi
	sleep __POLL__
done

echo "beet: $unit never came up serving release $expect" >&2
systemctl status "$unit" --no-pager --lines=40 >&2 || true
exit 1
"#,
			stack,
			&[
				("__DEPLOY_ID__", deploy_id),
				("__ATTEMPTS__", &attempts.to_string()),
				("__POLL__", &poll_secs.to_string()),
			],
		)
	}

	/// Render a shell script template, substituting `__TOKEN__` placeholders.
	///
	/// Placeholders rather than `format!` because these scripts are dense with
	/// `${..}` and `{ .. }`, which `format!` would need escaped into
	/// illegibility. `__APP__` is shared by every script, so it is substituted
	/// here rather than passed by each caller.
	fn render_script(
		template: &str,
		stack: &Stack,
		tokens: &[(&str, &str)],
	) -> String {
		tokens
			.iter()
			.fold(
				template.replace("__APP__", Self::service_name(stack)),
				|script, (token, value)| script.replace(token, value),
			)
			.trim_end()
			.to_string()
	}

	/// The identity of the box's machine config: a digest of the rendered
	/// cloud-init script, which is exactly what terraform replaces the instance
	/// on.
	///
	/// This is what the access key's rotation trigger keys on, so the key
	/// rotates with every machine-config change (a rebuild from a non-script
	/// change, ie a bundle resize, keeps its key). Keying it on the deploy id
	/// instead (as it once was) replaced the key every deploy, and since the
	/// instance interpolates the key into its `user_data` that alone forced a
	/// rebuild every deploy no matter what else was constant.
	///
	/// Not circular: the terraform *references* to the key are stable literals
	/// in this string, so hashing it says nothing about the key's value.
	fn machine_config_hash(user_data: &str) -> String {
		use sha2::Digest;
		sha2::Sha256::digest(user_data.as_bytes())
			.iter()
			.map(|byte| format!("{byte:02x}"))
			.collect()
	}
}

impl Block for LightsailBlock {
	fn artifact_label(&self) -> Option<&str> { Some(&self.label) }
	fn variables(&self) -> &[Variable] { &self.env_vars }

	fn apply_to_config(
		&self,
		_entity: &EntityRef,
		stack: &Stack,
		deployment: &Deployment,
		access: &AccessGrants,
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

		// One inline least-privilege policy naming exactly the resources this box
		// uses. Lightsail cannot carry an IAM role (a hard service limit, which is
		// why this block issues a static key at all), so the key is the whole
		// security boundary and its scope is the only thing that bounds a
		// compromise. It previously carried `AmazonS3FullAccess` +
		// `AmazonDynamoDBFullAccess`, ie every bucket and every table in the
		// account, readable from the metadata service and from the unit file.
		//
		// IAM Roles Anywhere would retire the static key entirely, but it needs a
		// CA, cert issuance and renewal, and a credential helper on a box that is
		// rebuilt every deploy. Judged disproportionate for two resources; scope
		// and rotation carry the weight instead.
		let policy_ident =
			stack.resource_ident(self.build_label("deploy-policy"));
		let policy = terra::ResourceDef::new_secondary(
			policy_ident.clone(),
			AwsIamUserPolicyDetails {
				name: Some(policy_ident.primary_identifier().clone()),
				user: user_name_ref.clone().into(),
				policy: self.runtime_policy(stack, deployment, access)?.into(),
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

		// the machine config, rendered once and used twice: as the instance's
		// user data, and as the identity the key rotation keys on.
		let user_data = self.build_user_data(
			stack,
			deployment,
			&access_key_id_ref,
			&access_key_secret_ref,
		)?;

		// Rotate the access key with every machine-config change, bounding a
		// leaked credential to the next rebuild rather than forever (see
		// `machine_config_hash`).
		//
		// The trigger cannot be the instance itself: the instance's user data
		// interpolates the key, so the instance already depends on the key and
		// pointing back at it would be a cycle. A `terraform_data` carrying the
		// machine config's digest gives the ordering rotation -> key -> instance,
		// and the coupling runs both ways: a machine change rotates the key, and
		// a rotated key changes the user data terraform renders, which replaces
		// the instance.
		let rotation_ident =
			stack.resource_ident(self.build_label("key-rotation"));
		let rotation_label = rotation_ident.label().to_string();
		config.add_untyped_resource(
			"terraform_data",
			&rotation_label,
			&json!({
				"input": Self::machine_config_hash(&user_data)
			}),
		)?;

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
		let mut instance_details = AwsLightsailInstanceDetails {
			availability_zone: self
				.availability_zone
				.clone()
				.unwrap_or_else(|| format!("{}a", stack.region()).into()),
			blueprint_id: self.blueprint_id.clone(),
			bundle_id: self.bundle_id.clone(),
			name: instance_ident.primary_identifier().clone(),
			key_pair_name: Some(keypair.field_ref("name").into()),
			user_data: Some(user_data),
			tags: Some(
				[
					(
						SmolStr::from("Project"),
						stack.app_name().unwrap_or_default().into(),
					),
					(SmolStr::from("Stage"), stack.stage().into()),
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
		let tcp_port = |port: u16| {
			AwsLightsailInstancePublicPortsResourceBlockTypePortInfo {
				from_port: port.into(),
				protocol: "tcp".into(),
				to_port: port.into(),
				..default()
			}
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
			.add_resource(&access_key)?
			.xmap(|config| {
				// rotate on the deploy-scoped trigger declared above
				config.set_lifecycle(
					"aws_iam_access_key",
					access_key.ident().label(),
					json!({
						"replace_triggered_by": [format!("terraform_data.{rotation_label}")]
					}),
				)
			})?
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
				self.emit_dns(
					stack,
					config,
					&static_ip.field_ref("ip_address"),
					false,
				)?;
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
		let (stack, deployment, dir) = Stack::default_local();
		let script = block
			.build_user_data(&stack, &deployment, "${key_id}", "${key_secret}")
			.unwrap();
		(script.to_string(), dir)
	}

	/// The rendered terraform config json for a block.
	fn build_json(block: &LightsailBlock) -> String {
		let (stack, deployment, _dir) = Stack::default_local();
		build_json_for(block, &stack, &deployment)
	}

	/// The rendered terraform config json for a block on a specific deploy,
	/// granting nothing (no resource blocks declared beside it).
	fn build_json_for(
		block: &LightsailBlock,
		stack: &Stack,
		deployment: &Deployment,
	) -> String {
		build_json_granting(block, stack, deployment, &[])
	}

	/// The rendered terraform config json for a block deployed alongside
	/// `declared`, whose grants it lowers into its runtime policy.
	fn build_json_granting(
		block: &LightsailBlock,
		stack: &Stack,
		deployment: &Deployment,
		declared: &[&dyn Block],
	) -> String {
		let mut world = World::new();
		let entity_mut = world.spawn(());
		let entity = entity_mut.as_readonly();
		stack
			.build_config(
				deployment,
				core::iter::once((entity.clone(), block as &dyn Block))
					.chain(
						declared.iter().map(|block| (entity.clone(), *block)),
					)
					.collect::<Vec<_>>(),
			)
			.unwrap()
			.to_json()
			.to_string()
	}

	/// The `terraform_data` input the access key's replacement is triggered by,
	/// ie the identity of the box's machine config.
	fn rotation_input(
		block: &LightsailBlock,
		stack: &Stack,
		deployment: &Deployment,
	) -> String {
		let label = stack
			.resource_ident(block.build_label("key-rotation"))
			.label()
			.to_string();
		serde_json::from_str::<serde_json::Value>(&build_json_for(
			block, stack, deployment,
		))
		.unwrap()["resource"]["terraform_data"][label]["input"]
			.as_str()
			.unwrap()
			.to_string()
	}

	/// Two deploys of the same machine config, ie the same box built twice with
	/// different code.
	fn two_deploys() -> (Stack, Deployment, Deployment, TestWorkDir) {
		let (stack, first, dir) = Stack::default_local();
		let second = first
			.clone()
			.with_deploy_id(uuid_ext::now_v7())
			.with_deploy_timestamp("2026-08-19T00:00:00Z".to_string());
		(stack, first, second, dir)
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

	/// A code-only deploy renders the SAME box, so terraform plans no change to
	/// the instance and the running unit is rolled onto the new binary instead
	/// (`<LightsailRelease/>`).
	///
	/// The whole terraform config is compared, not just the user data: anything
	/// deploy-varying anywhere in it would replace or update a resource, and the
	/// instance in particular cannot be updated in place at all. The box is
	/// burstable and starts every life with ZERO burst credit, so a rebuild does
	/// the heaviest work of the instance's life (a `dnf install`, an ~80MB pull,
	/// a Let's Encrypt issuance) clamped to the 20% baseline.
	#[beet_core::test]
	fn code_only_deploy_renders_one_box() {
		let (stack, first, second, _dir) = two_deploys();
		let block = LightsailBlock::default().with_allow_ssh(true);
		first.deploy_id().xpect_not_eq(*second.deploy_id());
		build_json_for(&block, &stack, &first)
			.xpect_eq(build_json_for(&block, &stack, &second));
		// the version it serves is resolved per start from the artifacts
		// bucket's stable pointer, and named nowhere in the machine config
		block
			.build_user_data(&stack, &first, "${key_id}", "${key_secret}")
			.unwrap()
			.as_str()
			.xpect_contains("current/main-lightsail.env")
			.xnot()
			.xpect_contains(&first.deploy_id().to_string())
			.xnot()
			.xpect_contains("Environment=BEET_DEPLOY_ID");
	}

	/// A machine-config change DOES rebuild the box, and rotates the access key
	/// with it, so a leaked credential is bounded by the next machine-config
	/// change rather than living forever.
	///
	/// Keying rotation on the deploy id instead (as it once was) replaced the
	/// key every deploy, and since the instance interpolates the key into its
	/// user data, that alone forced a rebuild every deploy no matter what else
	/// was constant.
	#[beet_core::test]
	fn machine_config_change_rebuilds_and_rotates() {
		let (stack, first, second, _dir) = two_deploys();
		let block = LightsailBlock::default();
		// same machine, two deploys: no rotation, so no new user data, so no
		// replacement
		rotation_input(&block, &stack, &first)
			.xpect_eq(rotation_input(&block, &stack, &second));
		// a different machine: a new key, and the user data that carries it
		rotation_input(&block.clone().with_allow_ssh(true), &stack, &first)
			.xpect_not_eq(rotation_input(&block, &stack, &first));
		rotation_input(&block.clone().with_app_port(9001), &stack, &first)
			.xpect_not_eq(rotation_input(&block, &stack, &first));
	}

	/// The box's fetch script and the artifacts client name the SAME pointer,
	/// and the launcher loads the identity that pointer publishes.
	///
	/// This is the one join between the two processes: the deploy writes the
	/// pointer, the machine reads it, and they never speak otherwise. A drift
	/// here is a box that boots the version it was built with forever.
	#[beet_core::test]
	fn machine_reads_the_pointer_the_deploy_writes() {
		let (script, _dir) = build_user_data(&LightsailBlock::default());
		script
			.as_str()
			.xpect_contains(
				&ArtifactLedger::release_pointer_key("main-lightsail")
					.to_string(),
			)
			.xpect_contains(&format!(
				"aws s3 cp \"s3://{}/${}\"",
				{
					let (stack, deployment, _dir) = Stack::default_local();
					deployment.artifact_bucket_name(&stack)
				},
				ArtifactLedger::ARTIFACT_KEY_VAR
			))
			// a pointer missing the key lands in the narrated keep-serving
			// path rather than aborting the fetch under `set -u`. Escaped
			// `$${..}`, since terraform reads the rendered script as a string
			// and a `:-` inside a real interpolation is a parse error, not a
			// mis-render: unescaped, `tofu validate` rejects the whole config.
			.xpect_contains(&format!(
				"[ -z \"$${{{}:-}}\" ]",
				ArtifactLedger::ARTIFACT_KEY_VAR
			))
			// the launcher exports the pointer's identity into the process
			.xpect_contains(". /etc/beet_infra/deploy.env");
	}

	/// Terraform reads `user_data` as a string, so every `${..}` in it is an
	/// interpolation. The only ones that may survive unescaped are the terraform
	/// refs this block deliberately injects (the access key pair and any declared
	/// variables); every shell expansion is escaped.
	///
	/// REGRESSION: an unescaped `${BEET_ARTIFACT_KEY:-}` in the release fetcher
	/// made `tofu validate` fail the whole config with "Extra characters after
	/// interpolation expression", ie no deploy at all.
	#[beet_core::test]
	fn escapes_shell_expansions_from_terraform() {
		let (stack, deployment, _dir) = Stack::default_local();
		let script = LightsailBlock::default()
			.build_user_data(&stack, &deployment, "${key_id}", "${key_secret}")
			.unwrap();
		// every unescaped `${` is one of the injected terraform refs
		script
			.match_indices("${")
			.filter(|(index, _)| !script[..*index].ends_with('$'))
			.map(|(index, _)| {
				script[index..].split('}').next().unwrap_or_default()
			})
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"${key_id",
				"${key_secret",
				"${key_id",
				"${key_secret",
			]);
	}

	/// The release step proves the RUNNING process carries the deploy's id,
	/// read out of its own environment.
	///
	/// A downloaded file, a zero-exit `systemctl restart` and an `active` unit
	/// all look like success while the box serves the previous binary or crash
	/// loops; only the live process settles it. It is also idempotent, so a
	/// freshly rebuilt box (which pulled this release at boot) is confirmed
	/// rather than bounced.
	#[beet_core::test]
	fn release_proves_the_running_process() {
		let (stack, _deployment, _dir) = Stack::default_local();
		LightsailBlock::default()
			.release_script(
				&stack,
				"my-deploy-id",
				Duration::from_secs(60),
				Duration::from_secs(5),
			)
			.as_str()
			.xpect_contains("expect=my-deploy-id")
			.xpect_contains("/proc/$pid/environ")
			.xpect_contains("s/^BEET_DEPLOY_ID=//p")
			.xpect_contains("unit=beet_infra.service")
			// idempotent: an already-current box is left alone
			.xpect_contains("beet: already serving $expect")
			// 60s of 5s attempts
			.xpect_contains("seq 1 12");
	}

	/// Secret env rides the unit's `Environment=` lines, never `ExecStart`.
	#[beet_core::test]
	fn secret_env_rides_environment_lines() {
		let (script, _dir) = build_user_data(
			&LightsailBlock::default()
				.with_secret_env("BEET_SSH_HOST_KEY", "abc123"),
		);
		script
			.as_str()
			.xpect_contains("Environment=BEET_SSH_HOST_KEY=abc123")
			.xnot()
			.xpect_contains("app abc123");
	}

	/// The IAM user carries ONE inline policy naming exactly the resources the
	/// box uses, and none of the account-wide managed policies.
	///
	/// Lightsail cannot carry an IAM role, so this key is the whole security
	/// boundary: it used to grant `AmazonS3FullAccess` + `AmazonDynamoDBFullAccess`,
	/// ie every bucket and table in the account, from a key readable via the
	/// instance metadata service and the unit file.
	#[beet_core::test]
	fn grants_only_least_privilege_policies() {
		let (stack, deployment, _dir) = Stack::default_local();
		let block = LightsailBlock::default();
		let json = build_json_granting(&block, &stack, &deployment, &[
			&S3BucketBlock::new("app").with_deploy_versioned(false),
			&DynamoTableBlock::new("analytics"),
		]);
		json.as_str()
			// scoped to the resources the stack DECLARED, resolved through the
			// one naming composition rather than restated on the block.
			.xpect_contains(&format!(
				"arn:aws:s3:::{}/*",
				stack.resource_name("app")
			))
			.xpect_contains(&format!(
				"arn:aws:s3:::{}/*",
				deployment.artifact_bucket_name(&stack)
			))
			.xpect_contains(&format!(
				"table/{}",
				stack.resource_name("analytics")
			))
			.xpect_contains(&block.log_group(&stack))
			// and never account-wide
			.xnot()
			.xpect_contains("AmazonS3FullAccess")
			.xnot()
			.xpect_contains("AmazonDynamoDBFullAccess")
			.xnot()
			.xpect_contains("CloudWatchAgentServerPolicy");
	}

	/// A stack declaring no table grants no DynamoDB access at all, rather than
	/// a wildcard table arn.
	#[beet_core::test]
	fn no_declared_table_grants_no_dynamo() {
		build_json(&LightsailBlock::default())
			.as_str()
			.xnot()
			.xpect_contains("dynamodb:");
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

	/// REGRESSION: Caddy is installed from the upstream static binary, never the
	/// cloudsmith rpm repo. Caddy publishes no `amzn` packages, but its repo
	/// script writes an `amzn/2023` baseurl with `skip_if_unavailable=1`, so
	/// `dnf install -y caddy` exited "No match for argument: caddy" and cloud-init
	/// carried on: the box booted serving the app on its own port with NOTHING on
	/// 80/443, and the deploy reported success while every hostname returned a
	/// Cloudflare 521. The install now writes its own unit and verifies the binary.
	#[cfg(feature = "cloudflare_dns")]
	#[beet_core::test]
	fn installs_caddy_from_static_release_not_rpm() {
		let (script, _dir) = build_user_data(
			&LightsailBlock::default()
				.with_dns(DnsProvider::cloudflare("example.org", "zone123")),
		);
		script
			.as_str()
			.xpect_contains(&format!(
				"caddy_{}_linux_amd64.tar.gz",
				LightsailBlock::CADDY_VERSION
			))
			.xpect_contains("/usr/local/bin/caddy version || exit 1")
			.xpect_contains("/etc/systemd/system/caddy.service")
			.xnot()
			.xpect_contains("cloudsmith")
			.xnot()
			.xpect_contains("dnf install -y caddy");
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

	/// Boot selection rides the launch `exec` as validated argv, with the boot
	/// route trailing every flag; ambient service config rides the unit's
	/// `Environment=` lines. The deployed stage is a platform binding: it flows
	/// from the stack, not the authored bootstrap.
	#[beet_core::test]
	fn splits_exec_and_env_channels() {
		let (stack, deployment, _dir) = Stack::default_local();
		let stack = stack.with_stage("staging");
		let script = LightsailBlock::default()
			.with_bootstrap(BootstrapConfig {
				store: Some(StoreUri::parse("s3://beet--dev--app").unwrap()),
				server: Some(RunningSetFilter::new("http")),
				..default()
			})
			.with_exec_route("serve")
			.build_user_data(&stack, &deployment, "${key_id}", "${key_secret}")
			.unwrap();
		script
			.as_str()
			.xpect_contains(
				"exec /opt/beet_infra/app --store=s3://beet--dev--app \
				--server=http serve",
			)
			// the unit runs the launcher, which resolves the release per start
			.xpect_contains("ExecStart=/usr/local/bin/beet_infra-run")
			.xpect_contains("Environment=BEET_STAGE=staging")
			// the store never leaks onto the env channel
			.xnot()
			.xpect_contains("BEET_STORE");
	}

	/// No boot route renders the bare invocation, ie an entry whose root boots
	/// its servers directly.
	#[beet_core::test]
	fn no_exec_route_renders_the_bare_invocation() {
		let (stack, deployment, _dir) = Stack::default_local();
		LightsailBlock::default()
			.with_bootstrap(BootstrapConfig {
				server: Some(RunningSetFilter::new("http")),
				..default()
			})
			.build_user_data(&stack, &deployment, "${key_id}", "${key_secret}")
			.unwrap()
			.as_str()
			.xpect_contains("exec /opt/beet_infra/app --server=http\n");
	}

	/// A token that cannot be rendered into a systemd `ExecStart` is a loud error
	/// rather than a corrupted unit file: the old space-join caveat, enforced.
	#[beet_core::test]
	fn rejects_unencodable_exec_args() {
		let (stack, deployment, _dir) = Stack::default_local();
		LightsailBlock::default()
			.with_bootstrap(BootstrapConfig {
				path: Some("/my page".into()),
				..default()
			})
			.build_user_data(&stack, &deployment, "id", "secret")
			.unwrap_err()
			.to_string()
			.xpect_contains("cannot be rendered");
	}

	// drives the native tofu Project, so it cannot compile for wasm
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test(timeout_ms = 120000)]
	#[ignore = "very slow"]
	async fn validate() {
		let (stack, deployment, _dir) = Stack::default_local();
		let block = LightsailBlock::default();
		let mut config = deployment.create_config(&stack);
		let mut world = World::new();
		block
			.apply_to_config(
				&world.spawn(()).as_readonly(),
				&stack,
				&deployment,
				&default(),
				&mut config,
			)
			.unwrap();
		let project = terra::Project::new(stack, deployment, config);
		project.validate().await.unwrap();
	}
}

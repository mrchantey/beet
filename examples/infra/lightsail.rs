//! Typed Lightsail infrastructure example.
//!
//! Run with:
//! ```sh
//!   cargo run --example lightsail --features=fs,rand,infra_aws_lightsail
//! ```

use beet::prelude::*;
use beet_infra::common_resources::aws_lightsail::*;
use beet_infra::types::config_exporter::ConfigExporter;
use beet_infra::types::config_exporter::Output;
use beet_infra::types::config_exporter::Variable;
use serde_json::json;

#[beet::main]
async fn main() -> Result {
	let app_name = "hello-lightsail";
	let stage = "dev";
	let prefix = format!("{}--{}", app_name, stage);

	// -- User data script ----------

	let user_data = format!(
		r#"#!/bin/bash
set -euo pipefail
mkdir -p /opt/{app_name}
cat > /etc/systemd/system/{app_name}.service <<'EOF'
[Unit]
Description=Hello Lightsail HTTP Server
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
"#,
		app_name = app_name
	);

	// -- Resources ----------------------------------------------------------

	let keypair = AwsLightsailKeyPairDetails {
		name_prefix: Some(format!("{}--keypair", prefix)),
		..Default::default()
	};

	let static_ip = AwsLightsailStaticIpDetails::new(format!("{}--ip", prefix));

	let mut instance = AwsLightsailInstanceDetails::new(
		"${var.availability_zone}".into(),
		"${var.blueprint_id}".into(),
		"${var.bundle_id}".into(),
		format!("{}--instance", prefix),
	);
	instance.key_pair_name =
		Some("${aws_lightsail_key_pair.keypair.name}".into());
	instance.user_data = Some(user_data);
	instance.tags = Some(
		[
			("Project".to_string(), app_name.to_string()),
			("Stage".to_string(), stage.to_string()),
		]
		.into_iter()
		.collect(),
	);

	let ip_attach = AwsLightsailStaticIpAttachmentDetails::new(
		"${aws_lightsail_instance.instance.name}".into(),
		"${aws_lightsail_static_ip.ip.name}".into(),
	);

	let mut ports = AwsLightsailInstancePublicPortsDetails::new(
		"${aws_lightsail_instance.instance.name}".into(),
	);
	ports.port_info = Some(vec![
		AwsLightsailInstancePublicPortsResourceBlockTypePortInfo::new(
			8080,
			"tcp".into(),
			8080,
		),
		AwsLightsailInstancePublicPortsResourceBlockTypePortInfo::new(
			22,
			"tcp".into(),
			22,
		),
	]);

	// -- Assemble the config ------------------------------------------------

	let exporter = ConfigExporter::new()
		// Typed resources — provider is registered automatically
		.with_resource("keypair", &keypair)
		.with_resource("ip", &static_ip)
		.with_resource("instance", &instance)
		.with_resource("ip_attach", &ip_attach)
		.with_resource("ports", &ports)
		// Variables
		.with_variable("availability_zone", Variable {
			r#type: Some("string".into()),
			default: Some(json!("us-east-1a")),
			description: Some("Lightsail availability zone".into()),
		})
		.with_variable("blueprint_id", Variable {
			r#type: Some("string".into()),
			default: Some(json!("amazon_linux_2023")),
			description: Some("Lightsail instance blueprint".into()),
		})
		.with_variable("bundle_id", Variable {
			r#type: Some("string".into()),
			default: Some(json!("nano_3_0")),
			description: Some("Lightsail instance bundle".into()),
		})
		.with_variable("server_port", Variable {
			r#type: Some("number".into()),
			default: Some(json!(8080)),
			description: Some("The port the server listens on".into()),
		})
		// Outputs
		.with_output("instance_name", Output {
			value: json!("${aws_lightsail_instance.instance.name}"),
			description: Some("The Lightsail instance name".into()),
			sensitive: None,
		})
		.with_output("static_ip_address", Output {
			value: json!("${aws_lightsail_static_ip.ip.ip_address}"),
			description: Some("The static IP address".into()),
			sensitive: None,
		});

	// -- Export & validate ---------------------------------------------------

	let dir = TempDir::new()?;
	let out_path = dir.join("main.tf.json");
	let result = exporter.export_and_validate(&out_path).await?;
	beet::cross_log!("tofu validate output: {}", result);

	Ok(())
}

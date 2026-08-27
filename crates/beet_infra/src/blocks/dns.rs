use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use serde_json::json;

/// A DNS record provider, embedded in a block that needs to publish a hostname
/// (a [`LambdaBlock`] gateway, a [`FargateBlock`] load balancer, a
/// [`LightsailBlock`] static IP). It emits a single record pointing its
/// `authority` at a target: a `CNAME` via [`emit`] for hostname targets, an
/// `A`/`AAAA` via [`emit_address`] for IP targets, plus any auxiliary records
/// (eg ACM DNS-validation) via [`emit_cname`].
///
/// [`emit`]: Self::emit
/// [`emit_address`]: Self::emit_address
///
/// [`emit_cname`]: Self::emit_cname
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub enum DnsProvider {
	/// A record in a Cloudflare zone. Authenticates from the
	/// `CLOUDFLARE_API_TOKEN` environment variable at apply time.
	#[cfg(feature = "cloudflare_dns")]
	Cloudflare {
		/// Fully-qualified record name, eg `dev.beet.org`.
		authority: SmolStr,
		/// The Cloudflare zone id (from `CLOUDFLARE_ZONE_ID`).
		zone_id: SmolStr,
		/// Whether to proxy through Cloudflare's edge. DNS-only (`false`) is
		/// required when the origin must be reached directly, eg raw TCP ssh
		/// or terminating TLS at the origin.
		proxied: bool,
	},
	/// A record in a Route53 hosted zone.
	Route53 {
		/// Fully-qualified record name.
		authority: SmolStr,
		/// The Route53 hosted zone id.
		zone_id: SmolStr,
	},
}

impl DnsProvider {
	/// A Cloudflare record, DNS-only (not proxied) by default.
	#[cfg(feature = "cloudflare_dns")]
	pub fn cloudflare(
		authority: impl Into<SmolStr>,
		zone_id: impl Into<SmolStr>,
	) -> Self {
		Self::Cloudflare {
			authority: authority.into(),
			zone_id: zone_id.into(),
			proxied: false,
		}
	}

	/// A Cloudflare record whose zone comes from `CLOUDFLARE_ZONE_ID`, the one
	/// place this repo has ever kept it (`ZoneAudit` and `CloudflareZoneSetup`
	/// both read it there, and the terraform provider block stays empty for the
	/// same reason the api token does).
	///
	/// `None` when the variable is unset, so a caller can say which zone it
	/// wanted in the error rather than emitting records into zone `""`.
	#[cfg(feature = "cloudflare_dns")]
	pub fn cloudflare_env(authority: impl Into<SmolStr>) -> Option<Self> {
		env_ext::var("CLOUDFLARE_ZONE_ID")
			.ok()
			.filter(|zone_id| !zone_id.is_empty())
			.map(|zone_id| Self::cloudflare(authority, zone_id))
	}

	/// A Route53 record.
	pub fn route53(
		authority: impl Into<SmolStr>,
		zone_id: impl Into<SmolStr>,
	) -> Self {
		Self::Route53 {
			authority: authority.into(),
			zone_id: zone_id.into(),
		}
	}

	/// Proxy a Cloudflare record through the edge (no effect on Route53).
	#[cfg(feature = "cloudflare_dns")]
	pub fn with_proxied(mut self, value: bool) -> Self {
		if let Self::Cloudflare { proxied, .. } = &mut self {
			*proxied = value;
		}
		self
	}

	/// The record name this provider publishes, eg `dev.beet.org`.
	pub fn authority(&self) -> &SmolStr {
		match self {
			#[cfg(feature = "cloudflare_dns")]
			Self::Cloudflare { authority, .. } => authority,
			Self::Route53 { authority, .. } => authority,
		}
	}

	/// The zone id the records are emitted into.
	pub fn zone_id(&self) -> &SmolStr {
		match self {
			#[cfg(feature = "cloudflare_dns")]
			Self::Cloudflare { zone_id, .. } => zone_id,
			Self::Route53 { zone_id, .. } => zone_id,
		}
	}

	/// Emit a `CNAME` pointing [`authority`](Self::authority) at `alias_target`
	/// (a terra field-ref like a load balancer's `dns_name` or an api gateway's
	/// `api_endpoint`). `label` is the resource label suffix.
	pub fn emit(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		label: &str,
		alias_target: &str,
	) -> Result {
		self.emit_record(
			stack,
			config,
			label,
			self.authority(),
			alias_target,
			self.proxied(),
			"CNAME",
		)?;
		Ok(())
	}

	/// Emit an address record pointing [`authority`](Self::authority) at
	/// `address` (a terra field-ref resolving to an IP, eg a Lightsail static
	/// IP's `ip_address`): an `AAAA` when `ipv6`, else an `A`.
	pub fn emit_address(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		label: &str,
		address: &str,
		ipv6: bool,
	) -> Result {
		self.emit_record(
			stack,
			config,
			label,
			self.authority(),
			address,
			self.proxied(),
			if ipv6 { "AAAA" } else { "A" },
		)?;
		Ok(())
	}

	/// Whether records emit proxied through Cloudflare's edge.
	fn proxied(&self) -> bool {
		#[cfg(feature = "cloudflare_dns")]
		return matches!(self, Self::Cloudflare { proxied: true, .. });
		#[cfg(not(feature = "cloudflare_dns"))]
		false
	}

	/// Emit a `CNAME` at an explicit `name` (always unproxied), pointing it at
	/// `target`. Both may be terra field-refs, which is how an ACM validation
	/// record reads its pair off `domain_validation_options` and how a mail
	/// domain's DKIM records read their selector off the SES identity. Returns
	/// the terraform resource address, for a `depends_on`.
	///
	/// Distinct from [`emit`](Self::emit), which publishes this provider's own
	/// [`authority`](Self::authority) and honours its proxy setting.
	pub fn emit_cname(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		label: &str,
		name: &str,
		target: &str,
	) -> Result<String> {
		self.emit_record(stack, config, label, name, target, false, "CNAME")
	}

	/// Emit a `TXT` record (SPF, DKIM, DMARC, MTA-STS, TLS-RPT, `_atproto`, …).
	/// Cloudflare takes the raw text; Route53 additionally requires the value
	/// double-quoted, the usual Route53 TXT convention (an already-quoted
	/// `value` is passed through unchanged).
	pub fn emit_txt(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		label: &str,
		name: &str,
		value: &str,
	) -> Result<String> {
		let content = match self {
			#[cfg(feature = "cloudflare_dns")]
			Self::Cloudflare { .. } => value.to_string(),
			Self::Route53 { .. } => quote_txt_value(value),
		};
		self.emit_record(stack, config, label, name, &content, false, "TXT")
	}

	/// Emit an `MX` record. Cloudflare carries `priority` as its own field
	/// alongside `content`; Route53 has no such field, so `priority target` is
	/// folded into the one content string it does have.
	pub fn emit_mx(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		label: &str,
		name: &str,
		priority: u16,
		target: &str,
	) -> Result<String> {
		match self {
			#[cfg(feature = "cloudflare_dns")]
			Self::Cloudflare { .. } => self.emit_record_with_priority(
				stack,
				config,
				label,
				name,
				target,
				false,
				"MX",
				Some(priority as i64),
			),
			Self::Route53 { .. } => self.emit_record(
				stack,
				config,
				label,
				name,
				&format!("{priority} {target}"),
				false,
				"MX",
			),
		}
	}

	/// Emit an `SRV` record. `name` is the full service name, eg
	/// `_jmap._tcp.stalwart.beetmash.com` (matching the "Name" column the DNS
	/// spec already writes it as); `target` is the resolving hostname.
	pub fn emit_srv(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		label: &str,
		name: &str,
		priority: u16,
		weight: u16,
		port: u16,
		target: &str,
	) -> Result<String> {
		let ident = stack.resource_ident(label);
		let address = match self {
			#[cfg(feature = "cloudflare_dns")]
			Self::Cloudflare { zone_id, .. } => {
				ensure_cloudflare_provider(config)?;
				let record = ResourceDef::new_secondary(
					ident,
					CloudflareDnsRecordDetails {
						name: name.into(),
						ttl: 1,
						r#type: "SRV".into(),
						zone_id: zone_id.clone(),
						data: Some(CloudflareDnsRecordData {
							priority: Some(priority as i64),
							weight: Some(weight as i64),
							port: Some(port as i64),
							target: Some(target.into()),
							..default()
						}),
						proxied: Some(false),
						..default()
					},
				);
				let address =
					format!("cloudflare_dns_record.{}", record.ident().label());
				config.add_resource(&record)?;
				address
			}
			Self::Route53 { zone_id, .. } => {
				let record = ResourceDef::new_secondary(
					ident,
					AwsRoute53RecordDetails {
						name: name.into(),
						r#type: "SRV".into(),
						zone_id: zone_id.clone(),
						ttl: Some(60),
						records: Some(vec![
							format!("{priority} {weight} {port} {target}")
								.into(),
						]),
						..default()
					},
				);
				let address =
					format!("aws_route53_record.{}", record.ident().label());
				config.add_resource(&record)?;
				address
			}
		};
		Ok(address)
	}

	/// Emit one record of `record_type` into this provider's zone, returning its
	/// terraform resource address (eg `cloudflare_dns_record.<label>`).
	fn emit_record(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		label: &str,
		name: &str,
		content: &str,
		proxied: bool,
		record_type: &str,
	) -> Result<String> {
		self.emit_record_with_priority(
			stack,
			config,
			label,
			name,
			content,
			proxied,
			record_type,
			None,
		)
	}

	/// [`Self::emit_record`], with an optional MX-style `priority` set on the
	/// Cloudflare record (Route53 has no such field: a caller that needs
	/// priority there folds it into `content` instead).
	fn emit_record_with_priority(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		label: &str,
		name: &str,
		content: &str,
		proxied: bool,
		record_type: &str,
		#[cfg_attr(not(feature = "cloudflare_dns"), allow(unused))]
		priority: Option<i64>,
	) -> Result<String> {
		let ident = stack.resource_ident(label);
		let address = match self {
			#[cfg(feature = "cloudflare_dns")]
			Self::Cloudflare { zone_id, .. } => {
				ensure_cloudflare_provider(config)?;
				let record = ResourceDef::new_secondary(
					ident,
					CloudflareDnsRecordDetails {
						name: name.into(),
						ttl: 1,
						r#type: record_type.into(),
						zone_id: zone_id.clone(),
						content: Some(content.into()),
						proxied: Some(proxied),
						priority,
						..default()
					},
				);
				let address =
					format!("cloudflare_dns_record.{}", record.ident().label());
				config.add_resource(&record)?;
				address
			}
			Self::Route53 { zone_id, .. } => {
				let record = ResourceDef::new_secondary(
					ident,
					AwsRoute53RecordDetails {
						name: name.into(),
						r#type: record_type.into(),
						zone_id: zone_id.clone(),
						ttl: Some(60),
						records: Some(vec![content.into()]),
						..default()
					},
				);
				let address =
					format!("aws_route53_record.{}", record.ident().label());
				config.add_resource(&record)?;
				address
			}
		};
		Ok(address)
	}
}

/// Route53 TXT record values must be double-quoted; an already-quoted value
/// is passed through unchanged.
fn quote_txt_value(value: &str) -> String {
	if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
		value.to_string()
	} else {
		format!("\"{}\"", value.replace('"', "\\\""))
	}
}

/// Ensure the Cloudflare terraform provider is configured. The block stays
/// empty: the provider authenticates from `CLOUDFLARE_API_TOKEN` in the
/// environment (inherited by the tofu subprocess), keeping the secret out of
/// `main.tf.json`.
#[cfg(feature = "cloudflare_dns")]
pub(crate) fn ensure_cloudflare_provider(config: &mut terra::Config) -> Result {
	config.ensure_provider_config(&terra::Provider::CLOUDFLARE, &json!({}))?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	/// TXT content rides straight through for Cloudflare, whose `content` field
	/// takes arbitrary text.
	#[cfg(feature = "cloudflare_dns")]
	#[beet_core::test]
	fn cloudflare_txt_is_unquoted() {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let mut config = deployment.create_config(&stack);
		DnsProvider::cloudflare("stalwart.beetmash.com", "zone123")
			.emit_txt(
				&stack,
				&mut config,
				"spf",
				"stalwart.beetmash.com",
				"v=spf1 include:amazonses.com -all",
			)
			.unwrap();
		config
			.to_json()
			.to_string()
			.xpect_contains("\"type\":\"TXT\"")
			.xpect_contains("v=spf1 include:amazonses.com -all")
			.xnot()
			.xpect_contains("\\\"v=spf1");
	}

	/// Route53 TXT values must be double-quoted, the usual Route53 convention;
	/// an unquoted `-all` SPF-style value is quoted going in.
	#[beet_core::test]
	fn route53_txt_is_quoted() {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let mut config = deployment.create_config(&stack);
		DnsProvider::route53("stalwart.beetmash.com", "zone123")
			.emit_txt(
				&stack,
				&mut config,
				"spf",
				"stalwart.beetmash.com",
				"v=spf1 include:amazonses.com -all",
			)
			.unwrap();
		config
			.to_json()
			.to_string()
			.xpect_contains("\\\"v=spf1 include:amazonses.com -all\\\"");
	}

	/// Cloudflare carries MX priority as its own field, separate from `content`.
	#[cfg(feature = "cloudflare_dns")]
	#[beet_core::test]
	fn cloudflare_mx_sets_priority_field() {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let mut config = deployment.create_config(&stack);
		DnsProvider::cloudflare("stalwart.beetmash.com", "zone123")
			.emit_mx(
				&stack,
				&mut config,
				"mx",
				"stalwart.beetmash.com",
				10,
				"mail.beetmash.com",
			)
			.unwrap();
		let json = config.to_json().to_string();
		json.xpect_contains("\"type\":\"MX\"")
			.xpect_contains("\"priority\":10")
			.xpect_contains("\"content\":\"mail.beetmash.com\"");
	}

	/// Route53 has no priority field, so `priority target` is folded into the
	/// one content string it does have.
	#[beet_core::test]
	fn route53_mx_folds_priority_into_content() {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let mut config = deployment.create_config(&stack);
		DnsProvider::route53("stalwart.beetmash.com", "zone123")
			.emit_mx(
				&stack,
				&mut config,
				"mx",
				"stalwart.beetmash.com",
				10,
				"mail.beetmash.com",
			)
			.unwrap();
		config
			.to_json()
			.to_string()
			.xpect_contains("\"10 mail.beetmash.com\"");
	}

	/// Cloudflare structures SRV fields under `data`, never touching `content`.
	#[cfg(feature = "cloudflare_dns")]
	#[beet_core::test]
	fn cloudflare_srv_uses_data_block() {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let mut config = deployment.create_config(&stack);
		DnsProvider::cloudflare("zone.example", "zone123")
			.emit_srv(
				&stack,
				&mut config,
				"jmap",
				"_jmap._tcp.stalwart.beetmash.com",
				0,
				1,
				443,
				"mail.beetmash.com",
			)
			.unwrap();
		let json = config.to_json().to_string();
		json.xpect_contains("\"type\":\"SRV\"")
			.xpect_contains("\"priority\":0")
			.xpect_contains("\"weight\":1")
			.xpect_contains("\"port\":443")
			.xpect_contains("\"target\":\"mail.beetmash.com\"");
	}

	/// Route53 has no `data` block, so `priority weight port target` is folded
	/// into the one content string it does have.
	#[beet_core::test]
	fn route53_srv_folds_fields_into_content() {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let mut config = deployment.create_config(&stack);
		DnsProvider::route53("zone.example", "zone123")
			.emit_srv(
				&stack,
				&mut config,
				"jmap",
				"_jmap._tcp.stalwart.beetmash.com",
				0,
				1,
				443,
				"mail.beetmash.com",
			)
			.unwrap();
		config
			.to_json()
			.to_string()
			.xpect_contains("\"0 1 443 mail.beetmash.com\"");
	}
}

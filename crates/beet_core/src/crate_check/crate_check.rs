use crate::prelude::*;

/// Declares crate requirements the running binary must satisfy, checked on
/// insert against the spawned [`CrateRegistration`] set: every failure is
/// collected, printed as an alphabetical list, and the app exits non-zero.
///
/// Unprefixed items check the primary registration (the `beet` binary);
/// `crate/feature` and `crate@version` items check that crate:
///
/// ```html
/// <CrateCheck features="sockets,thread,beet_esp/alvik" versions="0.0.9,beet_esp@0.5.9"/>
/// ```
///
/// An entry document declaring its requirements this way fails fast with the
/// full missing list, instead of degrading into unresolved tags at load time.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct CrateCheck {
	/// Comma-separated required features, each `feature` or `crate/feature`.
	pub features: SmolStr,
	/// Comma-separated minimum versions, each `x.y.z` or `crate@x.y.z`.
	pub versions: SmolStr,
}

impl CrateCheck {
	/// Require `features`, each `feature` or `crate/feature`, comma-separated.
	pub fn features(features: impl Into<SmolStr>) -> Self {
		Self {
			features: features.into(),
			versions: default(),
		}
	}

	/// Collect every `<CrateCheck>` element in a parsed entry tree, the
	/// registry-free pre-scan entry building runs so a check fires even when
	/// the tree itself fails to build (eg its root tag is feature-gated out).
	#[cfg(feature = "bsx")]
	pub fn extract_checks(nodes: &[BsxNode]) -> Vec<Self> {
		let mut checks = Vec::new();
		Self::collect_checks(nodes, &mut checks);
		checks
	}

	/// Recursively collect `<CrateCheck>` declarations from `nodes`.
	#[cfg(feature = "bsx")]
	fn collect_checks(nodes: &[BsxNode], checks: &mut Vec<Self>) {
		for node in nodes {
			let BsxNode::Element(element) = node else {
				continue;
			};
			if element.tag == "CrateCheck" {
				let mut check = Self::default();
				for attr in &element.attributes {
					if let AttrValue::Str(value) = &attr.value {
						match attr.key.as_str() {
							"features" => check.features = value.into(),
							"versions" => check.versions = value.into(),
							_ => {}
						}
					}
				}
				checks.push(check);
			}
			Self::collect_checks(&element.children, checks);
		}
	}

	/// Every failed requirement as a sorted list, empty when all pass.
	pub fn failures<'a>(
		&self,
		registrations: impl IntoIterator<Item = &'a CrateRegistration> + Clone,
	) -> Vec<String> {
		let mut failures = Vec::new();
		// feature items: `feature` or `crate/feature`
		for item in Self::split_items(&self.features) {
			let (crate_name, feature) = match item.split_once('/') {
				Some((crate_name, feature)) => (Some(crate_name), feature),
				None => (None, item),
			};
			match Self::registration(registrations.clone(), crate_name) {
				None => failures.push(Self::missing_crate(crate_name)),
				Some(registration) if !registration.has_feature(feature) => {
					failures.push(format!(
						"{}/{feature} (feature not compiled in)",
						registration.crate_name()
					))
				}
				Some(_) => {}
			}
		}
		// version items: `x.y.z` or `crate@x.y.z`
		for item in Self::split_items(&self.versions) {
			let (crate_name, version) = match item.split_once('@') {
				Some((crate_name, version)) => (Some(crate_name), version),
				None => (None, item),
			};
			match Self::registration(registrations.clone(), crate_name) {
				None => failures.push(Self::missing_crate(crate_name)),
				Some(registration)
					if parse_version(registration.version())
						< parse_version(version) =>
				{
					failures.push(format!(
						"{}@{version} (installed {})",
						registration.crate_name(),
						registration.version()
					))
				}
				Some(_) => {}
			}
		}
		failures.sort();
		failures.dedup();
		failures
	}

	/// Split a comma-separated requirement list, skipping empty segments.
	fn split_items(list: &str) -> impl Iterator<Item = &str> {
		list.split(',').map(str::trim).filter(|item| !item.is_empty())
	}

	/// The registration for `crate_name`, or the primary one when unprefixed.
	fn registration<'a>(
		registrations: impl IntoIterator<Item = &'a CrateRegistration>,
		crate_name: Option<&str>,
	) -> Option<&'a CrateRegistration> {
		registrations.into_iter().find(|reg| match crate_name {
			Some(name) => reg.crate_name() == name,
			None => reg.skip_prefix(),
		})
	}

	/// The failure line for an unregistered crate.
	fn missing_crate(crate_name: Option<&str>) -> String {
		match crate_name {
			Some(name) => format!("{name} (crate not registered in this binary)"),
			None => "no primary crate registration in this binary (the binary \
				must spawn a `crate_registration!`)"
				.to_string(),
		}
	}

	/// Observer: validate an inserted [`CrateCheck`] against every spawned
	/// [`CrateRegistration`], logging all failures alphabetically and exiting
	/// non-zero on any. Failures across checks inserted in the same frame all
	/// report before the exit processes in `Last`.
	pub fn check_on_insert(
		ev: On<Insert, CrateCheck>,
		checks: Query<&CrateCheck>,
		registrations: Query<&CrateRegistration>,
		mut commands: Commands,
	) -> Result {
		let failures = checks.get(ev.entity)?.failures(&registrations);
		if failures.is_empty() {
			return Ok(());
		}
		error!(
			"crate check failed, this binary is missing:\n  {}\nrebuild with \
			the missing features, ie `cargo install --path crates/beet-cli \
			--all-features`",
			failures.join("\n  ")
		);
		commands.write_message(AppExit::error());
		Ok(())
	}
}

/// A lenient semver triple for ordering, ignoring pre-release/build suffixes.
fn parse_version(version: &str) -> (u32, u32, u32) {
	let mut parts = version
		.split('.')
		.map(|part| {
			part.split(|char: char| !char.is_ascii_digit())
				.next()
				.and_then(|digits| digits.parse().ok())
				.unwrap_or(0)
		})
		.chain(core::iter::repeat(0));
	(
		parts.next().unwrap_or(0),
		parts.next().unwrap_or(0),
		parts.next().unwrap_or(0),
	)
}

/// Registers the crate-check components and the insert-time validation, so an
/// entry's `<CrateCheck/>` verifies the running binary was compiled with the
/// features it needs. Always on via `BeetPlugins`.
#[derive(Default)]
pub struct CrateCheckPlugin;

impl Plugin for CrateCheckPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<CrateRegistration>()
			.register_type::<CrateCheck>()
			.add_observer(CrateCheck::check_on_insert);
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	fn registrations() -> Vec<CrateRegistration> {
		vec![
			CrateRegistration::new("beet-cli", "0.0.9")
				.with_feature("sockets")
				.with_skip_prefix(),
			CrateRegistration::new("beet_esp", "0.5.3").with_feature("alvik"),
		]
	}

	#[crate::test]
	fn passes_when_satisfied() {
		CrateCheck {
			features: "sockets,beet_esp/alvik".into(),
			versions: "0.0.9,beet_esp@0.5.3".into(),
		}
		.failures(&registrations())
		.xpect_empty();
	}

	#[crate::test]
	fn collects_all_failures_alphabetically() {
		CrateCheck {
			features: "thread,sockets,beet_esp/motors".into(),
			versions: "beet_esp@0.6.0".into(),
		}
		.failures(&registrations())
		.xpect_eq(vec![
			"beet-cli/thread (feature not compiled in)".to_string(),
			"beet_esp/motors (feature not compiled in)".to_string(),
			"beet_esp@0.6.0 (installed 0.5.3)".to_string(),
		]);
	}

	#[crate::test]
	fn reports_unregistered_crates() {
		CrateCheck::features("other-crate/foo")
			.failures(&registrations())
			.xpect_eq(vec![
				"other-crate (crate not registered in this binary)".to_string(),
			]);
	}

	#[crate::test]
	fn version_ordering_is_lenient() {
		use super::parse_version;
		parse_version("1.2.3").xpect_eq((1, 2, 3));
		parse_version("0.5").xpect_eq((0, 5, 0));
		parse_version("1.0.0-rc.1").xpect_eq((1, 0, 0));
		(parse_version("0.10.0") > parse_version("0.9.9")).xpect_true();
	}

	#[cfg(feature = "bsx")]
	#[crate::test]
	fn extracts_checks_from_raw_markup() {
		let nodes = parse_document(
			"<NotARegisteredTag><CrateCheck features=\"sockets\" versions=\"0.1.0\"/></NotARegisteredTag>",
			&BsxParseConfig::bsx(),
		)
		.unwrap();
		let checks = CrateCheck::extract_checks(&nodes);
		checks.len().xpect_eq(1);
		checks[0].features.xpect_eq("sockets");
		checks[0].versions.xpect_eq("0.1.0");
	}

	#[crate::test]
	fn failing_check_writes_app_exit() {
		let mut app = App::new();
		app.add_plugins(super::CrateCheckPlugin);
		app.world_mut().spawn(
			CrateRegistration::new("beet-cli", "0.0.9").with_skip_prefix(),
		);
		app.world_mut().spawn(CrateCheck::features("sockets"));
		app.update();
		app.world_mut()
			.resource_mut::<Messages<AppExit>>()
			.drain()
			.count()
			.xpect_eq(1);
	}
}

//! [`Version`] and [`VersionReq`], the one semver pair every version in beet
//! goes by: a package's, a compiled crate's, a tool's.
use crate::prelude::*;
use core::str::FromStr;

/// A semver version, `MAJOR.MINOR.PATCH` with an optional `-prerelease` and
/// `+build`, ordered by semver precedence, so `0.10.0` follows `0.9.0` and a
/// prerelease precedes its release.
///
/// ```
/// # use beet_core::prelude::*;
/// let version = Version::parse("0.10.0").unwrap();
/// (version > Version::parse("0.9.0").unwrap()).xpect_true();
/// Version::parse("0.1.2-alpha.3").unwrap().is_prerelease().xpect_true();
/// Version::parse("0.1").unwrap_err();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[reflect(opaque, Debug, Default, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct Version(semver::Version);

impl Version {
	/// The version a package declaring none compiles at, `0.0.0`.
	pub const IMPLICIT: Self = Self::new(0, 0, 0);

	/// `MAJOR.MINOR.PATCH` with no prerelease or build.
	pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
		Self(semver::Version::new(major, minor, patch))
	}

	/// Parse `MAJOR.MINOR.PATCH`, a `-prerelease` and a `+build` allowed.
	pub fn parse(text: &str) -> Result<Self> {
		semver::Version::parse(text).map(Self).map_err(|err| {
			bevyhow!(
				"`{text}` is no version: {err}; semver, ie `0.1.2` or \
				 `0.1.2-alpha.3`"
			)
		})
	}

	/// The major version.
	pub fn major(&self) -> u64 { self.0.major }
	/// The minor version.
	pub fn minor(&self) -> u64 { self.0.minor }
	/// The patch version.
	pub fn patch(&self) -> u64 { self.0.patch }

	/// Whether this is a prerelease, which `latest` never names.
	pub fn is_prerelease(&self) -> bool { !self.0.pre.is_empty() }
}

impl Default for Version {
	fn default() -> Self { Self::IMPLICIT }
}

impl core::fmt::Display for Version {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.0.fmt(f)
	}
}

impl FromStr for Version {
	type Err = BevyError;
	fn from_str(text: &str) -> Result<Self> { Self::parse(text) }
}

impl TryFrom<SmolStr> for Version {
	type Error = BevyError;
	fn try_from(text: SmolStr) -> Result<Self> { Self::parse(&text) }
}

impl From<Version> for SmolStr {
	fn from(version: Version) -> Self { version.to_string().into() }
}

/// A version requirement in Cargo's grammar: `0.1` (caret), `~0.1.2`,
/// `0.1.*`, `=0.1.2`, `>=0.1, <0.3`. Written back normalized, so `0.1` reads
/// `^0.1`.
///
/// ```
/// # use beet_core::prelude::*;
/// let requirement = VersionReq::parse("0.1").unwrap();
/// requirement.matches(&Version::parse("0.1.9").unwrap()).xpect_true();
/// requirement.matches(&Version::parse("0.2.0").unwrap()).xpect_false();
/// requirement.to_string().xpect_eq("^0.1");
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect)]
#[reflect(opaque, Debug, Default, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct VersionReq(semver::VersionReq);

impl VersionReq {
	/// Parse a requirement in Cargo's grammar.
	pub fn parse(text: &str) -> Result<Self> {
		semver::VersionReq::parse(text).map(Self).map_err(|err| {
			bevyhow!(
				"`{text}` is no version requirement: {err}; Cargo's grammar, \
				 ie `0.1`, `0.1.*`, `=0.1.2`"
			)
		})
	}

	/// Whether `version` satisfies this requirement. A prerelease satisfies
	/// only a requirement naming a prerelease of the same `MAJOR.MINOR.PATCH`,
	/// as in Cargo.
	pub fn matches(&self, version: &Version) -> bool {
		self.0.matches(&version.0)
	}
}

impl core::fmt::Display for VersionReq {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.0.fmt(f)
	}
}

impl FromStr for VersionReq {
	type Err = BevyError;
	fn from_str(text: &str) -> Result<Self> { Self::parse(text) }
}

impl TryFrom<SmolStr> for VersionReq {
	type Error = BevyError;
	fn try_from(text: SmolStr) -> Result<Self> { Self::parse(&text) }
}

impl From<VersionReq> for SmolStr {
	fn from(requirement: VersionReq) -> Self { requirement.to_string().into() }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn orders_by_precedence() {
		let version = |text: &str| Version::parse(text).unwrap();
		(version("0.0.9-dev.16") < version("0.0.9")).xpect_true();
		(version("0.10.0") > version("0.9.9")).xpect_true();
		(version("1.0.0-alpha") < version("1.0.0-beta")).xpect_true();
	}

	#[crate::test]
	fn validates_versions() {
		Version::parse("1.2.3+build.1")
			.unwrap()
			.is_prerelease()
			.xpect_false();
		Version::parse("01.2.3").unwrap_err();
		Version::parse("1.2.x").unwrap_err();
	}

	#[cfg(feature = "serde")]
	#[crate::test]
	fn serializes_as_its_text() {
		let version = Version::parse("0.1.2-rc.1").unwrap();
		let value = Value::from_serde(&version).unwrap();
		value.xpect_eq(Value::Str("0.1.2-rc.1".into()));
		value.into_serde::<Version>().unwrap().xpect_eq(version);
	}
}

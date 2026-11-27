//! The validated names a package, its branches and runtimes go by, and its
//! address; its [`Version`] is beet's one semver type.
use crate::prelude::*;

/// A package's name: lowercase `a-z0-9` and dashes, starting with a letter,
/// never a dot, so it is a url segment, an rkey prefix and a space key as it
/// stands. No name is reserved.
///
/// ```
/// # use beet_core::prelude::*;
/// PackageName::parse("todo-list").unwrap();
/// PackageName::parse("Todo").unwrap_err();
/// PackageName::from_dir_name("My Site 2").as_str().xpect_eq("my-site-2");
/// ```
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, StringPrimitive,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct PackageName(SmolStr);

impl PackageName {
	/// Parse a name, refusing any but lowercase letters, digits and dashes
	/// starting with a letter.
	pub fn parse(text: &str) -> Result<Self> {
		let valid = text.starts_with(|c: char| c.is_ascii_lowercase())
			&& text.bytes().all(|byte| {
				byte.is_ascii_lowercase()
					|| byte.is_ascii_digit()
					|| byte == b'-'
			});
		match valid {
			true => Self(SmolStr::new(text)).xok(),
			false => bevybail!(
				"`{text}` is no package name: lowercase `a-z0-9` and dashes, \
				 starting with a letter"
			),
		}
	}

	/// The name an implicit package takes from its directory: lowercased,
	/// every run of other characters a dash, `package` when nothing is left.
	pub fn from_dir_name(dir: &str) -> Self {
		let mut name = String::new();
		for char in dir.chars().map(|char| char.to_ascii_lowercase()) {
			match char.is_ascii_lowercase() || char.is_ascii_digit() {
				true => name.push(char),
				false if !name.is_empty() && !name.ends_with('-') => {
					name.push('-')
				}
				false => {}
			}
		}
		let name = name
			.trim_matches('-')
			.trim_start_matches(|c: char| c.is_ascii_digit() || c == '-');
		match name.is_empty() {
			true => Self(SmolStr::new_static("package")),
			false => Self(SmolStr::new(name)),
		}
	}
}

/// A branch's name, the rules of a [`PackageName`]. `main` is the default.
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, StringPrimitive,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct BranchName(SmolStr);

impl BranchName {
	/// The branch a compile writes when the launch names none.
	pub const MAIN: Self = Self(SmolStr::new_inline("main"));

	/// Parse a name under a [`PackageName`]'s rules.
	pub fn parse(text: &str) -> Result<Self> {
		PackageName::parse(text)
			.map(|name| Self(name.0))
			.map_err(|_| {
				bevyhow!(
					"`{text}` is no branch name: lowercase `a-z0-9` and \
					 dashes, starting with a letter"
				)
			})
	}
}

impl Default for BranchName {
	fn default() -> Self { Self::MAIN }
}

/// A runtime's name, the rules of a [`PackageName`].
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, StringPrimitive,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct RuntimeName(SmolStr);

impl RuntimeName {
	/// Parse a name under a [`PackageName`]'s rules.
	pub fn parse(text: &str) -> Result<Self> {
		PackageName::parse(text).map(|name| Self(name.0))
	}
}

/// An Automerge change hash; a set of them is a point in a document's
/// history.
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, StringPrimitive,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct ChangeHash(SmolStr);

impl ChangeHash {
	/// Parse a hash, hex digits only.
	pub fn parse(text: &str) -> Result<Self> {
		match !text.is_empty()
			&& text.bytes().all(|byte| byte.is_ascii_hexdigit())
		{
			true => Self(SmolStr::new(text)).xok(),
			false => bevybail!("`{text}` is no change hash: hex digits"),
		}
	}
}

/// A package's address. A leading `@` separates an account, a handle or a
/// did, from one of this account's package names.
///
/// ```text
/// my-package               a package on this account, its latest release
/// my-package@0.1           a Cargo requirement, caret
/// my-package#main          a branch's package record, for development
/// @beet.org                beet.org's home package, the one its home record names
/// @beet.org/foo-bar@0.1
/// @did:plc:abc/foo-bar
/// ```
///
/// Parsed from the right: `#branch`, then `@requirement`, then `/name`, then
/// the account. Absent both a requirement and a branch it means the
/// package's `latest` release.
///
/// ```
/// # use beet_core::prelude::*;
/// let address = PackageAddress::parse("@beet.org/foo-bar@0.1").unwrap();
/// let parts = address.parts();
/// parts.account.unwrap().as_str().xpect_eq("beet.org");
/// parts.name.unwrap().as_str().xpect_eq("foo-bar");
/// parts.requirement.unwrap().to_string().xpect_eq("^0.1");
/// PackageAddress::parse("my-package#main").unwrap().parts().branch.unwrap()
/// 	.xpect_eq(BranchName::MAIN);
/// ```
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, StringPrimitive,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct PackageAddress(SmolStr);

/// A [`PackageAddress`] read apart.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PackageAddressParts {
	/// The account, a handle or a did, absent for this account's own.
	pub account: Option<SmolStr>,
	/// The package, absent for an account's home package.
	pub name: Option<PackageName>,
	/// A version requirement.
	pub requirement: Option<VersionReq>,
	/// A branch, for development.
	pub branch: Option<BranchName>,
}

impl PackageAddress {
	/// Parse an address, see the type docs for its forms.
	pub fn parse(text: &str) -> Result<Self> {
		Self::read(text)?;
		Self(SmolStr::new(text)).xok()
	}

	/// The address read apart.
	pub fn parts(&self) -> PackageAddressParts {
		Self::read(&self.0).unwrap_or_default()
	}

	fn read(text: &str) -> Result<PackageAddressParts> {
		let mut parts = PackageAddressParts::default();
		let (rest, branch) = match text.rsplit_once('#') {
			Some((rest, branch)) => (rest, Some(BranchName::parse(branch)?)),
			None => (text, None),
		};
		parts.branch = branch;
		let (account, package) = match rest.strip_prefix('@') {
			Some(rest) => match rest.split_once('/') {
				Some((account, package)) => (Some(account), Some(package)),
				None => (Some(rest), None),
			},
			None => (None, Some(rest)),
		};
		if let Some(account) = account {
			if account.is_empty() {
				bevybail!("`{text}` names no account after its `@`");
			}
			parts.account = Some(SmolStr::new(account));
		}
		if let Some(package) = package {
			let (name, requirement) = match package.split_once('@') {
				Some((name, requirement)) => {
					(name, Some(VersionReq::parse(requirement)?))
				}
				None => (package, None),
			};
			parts.name = Some(PackageName::parse(name)?);
			parts.requirement = requirement;
		}
		if parts.name.is_none()
			&& (parts.requirement.is_some() || parts.branch.is_some())
		{
			bevybail!(
				"`{text}` names an account's home package, which takes no \
				 requirement or branch"
			);
		}
		parts.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn reads_every_address_form() {
		let parts = |text: &str| PackageAddress::parse(text).unwrap().parts();
		parts("my-package")
			.name
			.unwrap()
			.as_str()
			.xpect_eq("my-package");
		parts("@beet.org").name.xpect_none();
		parts("@did:plc:abc/foo-bar")
			.account
			.unwrap()
			.as_str()
			.xpect_eq("did:plc:abc");
		PackageAddress::parse("@beet.org#main").unwrap_err();
		PackageAddress::parse("Bad").unwrap_err();
	}
}

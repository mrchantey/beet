//! [`Package`] and the children declaring what it exports, depends on, imports
//! and ships, each hoisted by its [`SceneMeta`] onto the entry's scene, its
//! branch record, its package record and its releases.
use crate::prelude::*;
use alloc::collections::BTreeMap;

/// `org.beet.core#package`, authored on the entry document's root: a directory
/// holding one is a package, named whatever this says. The identity of a
/// release, and hoisted beside `Branch` on a branch record and a package
/// record: one component, every level.
///
/// ```html
/// <Package name="todos" version="0.1.0">
/// 	<Export key="." path="widgets/"/>
/// 	<Dependency package="@beet.org/widgets@0.1"/>
/// 	<Import name="." from="widgets"/>
/// </Package>
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect, LexiconDef)]
#[reflect(Component, LexiconDef, SceneMeta, Serialize, Deserialize)]
#[derive(Serialize, Deserialize)]
pub struct Package {
	/// The package's name, its space's key and every record's rkey prefix.
	pub name: PackageName,
	/// The version a release freezes it at.
	pub version: Version,
	/// The dependencies whose exports appear unprefixed, hoisted from the
	/// [`Import`] children.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[reflect(default)]
	pub imports: Vec<Import>,
}

impl Package {
	/// A package named `name` at `version`.
	pub fn new(name: PackageName, version: Version) -> Self {
		Self {
			name,
			version,
			imports: default(),
		}
	}

	/// The package a directory with no declaration compiles as, named for the
	/// directory at [`Version::IMPLICIT`].
	pub fn implicit(dir_name: &str) -> Self {
		Self::new(PackageName::from_dir_name(dir_name), Version::IMPLICIT)
	}

	/// The meta entity's `Package`, cloned from the declaration at `entity`
	/// where none was hoisted yet, so the declaration and its children hoist
	/// in any order.
	fn hoisted<'a>(
		cx: &'a mut SceneMetaContext,
		declaration: Entity,
	) -> Result<Mut<'a, Package>> {
		let meta = cx.meta;
		if !cx.world.entity(meta).contains::<Package>() {
			let Some(package) = cx.world.entity(declaration).get::<Package>()
			else {
				return Err(cx.error("expected a `Package` declaration"));
			};
			let package =
				Package::new(package.name.clone(), package.version.clone());
			cx.world.entity_mut(meta).insert(package);
		}
		cx.world
			.get_mut::<Package>(meta)
			.expect("inserted just above")
			.xok()
	}

	/// The `Package` a child declaration sits under, an error naming the
	/// child when it sits elsewhere.
	fn parent_of(cx: &SceneMetaContext, child: &str) -> Result<Entity> {
		cx.parent()
			.filter(|parent| cx.world.entity(*parent).contains::<Package>())
			.ok_or_else(|| {
				cx.error(format!("`{child}` is a child of `Package`"))
			})
	}
}

impl SceneMeta for Package {
	fn apply(cx: &mut SceneMetaContext) -> Result {
		let entity = cx.entity;
		let declared = cx.get::<Self>()?.clone();
		let mut package = Self::hoisted(cx, entity)?;
		package.name = declared.name;
		package.version = declared.version;
		Ok(())
	}
}

/// One export, a child of [`Package`]: package.json's `exports` in shape, a key
/// (`.` for the top of the namespace, a prefix, `*` patterns allowed) mapping
/// to a path, any file. A document is private unless exported.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default, SceneMeta)]
pub struct Export {
	/// What a dependent names, ie `.` or `widgets/*`.
	pub key: SmolStr,
	/// The file it names in this package.
	pub path: RelPath,
}

impl SceneMeta for Export {
	fn apply(cx: &mut SceneMetaContext) -> Result {
		Package::parent_of(cx, "Export")?;
		let export = cx.get::<Self>()?.clone();
		let error = cx.error(format!("`{}` is exported twice", export.key));
		match cx.meta_mut::<Exports>().0.insert(export.key, export.path) {
			Some(_) => Err(error),
			None => Ok(()),
		}
	}
}

/// `org.beet.core#exports`, hoisted from the [`Export`] children of
/// [`Package`].
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	Component,
	Reflect,
	LexiconDef,
	Serialize,
	Deserialize,
)]
#[reflect(Component, Default, LexiconDef, Serialize, Deserialize)]
pub struct Exports(pub BTreeMap<SmolStr, RelPath>);

/// One dependency, a child of [`Package`]: by address, by path, or both, the
/// path while developing and the address for everyone else. Keyed in
/// [`Dependencies`] by the prefix documents use ([`key`](Self::key)).
///
/// ```html
/// <Dependency package="@beet.org/widgets@0.1"/>
/// <Dependency alias="ui" path="../widgets"/>
/// ```
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	Component,
	Reflect,
	Serialize,
	Deserialize,
)]
#[reflect(Component, Default, SceneMeta, Serialize, Deserialize)]
pub struct Dependency {
	/// The prefix documents name it by, where it is not the package's name.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub alias: Option<PackageName>,
	/// Where it is published.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub package: Option<PackageAddress>,
	/// A local directory relative to this package, for development, ie
	/// `../widgets`, within the same source directory.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub path: Option<SmolPath>,
	/// What it resolved to, written by the compiler.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub resolved: Option<ResolvedDependency>,
}

impl Dependency {
	/// The prefix documents name it by: its alias, else the package its
	/// address names, else its path's directory.
	///
	/// ## Errors
	/// Errors when none of those is a package name, ie a home package by
	/// handle, which takes an alias.
	pub fn key(&self) -> Result<PackageName> {
		if let Some(alias) = &self.alias {
			return alias.clone().xok();
		}
		if let Some(address) = &self.package {
			return address.parts().name.ok_or_else(|| {
				bevyhow!(
					"the dependency `{address}` names a home package, so it \
					 takes an alias: `<Dependency alias=\"..\" ..>`"
				)
			});
		}
		match &self.path {
			Some(path) => {
				PackageName::parse(path.file_name().unwrap_or_default())
					.map_err(|_| {
						bevyhow!(
							"the dependency at `{path}` takes an alias, its directory \
					 being no package name: `<Dependency alias=\"..\" ..>`"
						)
					})
			}
			None => {
				bevybail!("a dependency names a `package`, a `path` or both")
			}
		}
	}
}

impl SceneMeta for Dependency {
	fn apply(cx: &mut SceneMetaContext) -> Result {
		Package::parent_of(cx, "Dependency")?;
		let dependency = cx.get::<Self>()?.clone();
		let key = dependency.key().map_err(|err| cx.error(err))?;
		let error = cx.error(format!("`{key}` is a dependency twice"));
		match cx.meta_mut::<Dependencies>().0.insert(key, dependency) {
			Some(_) => Err(error),
			None => Ok(()),
		}
	}
}

/// `org.beet.core#dependencies`, hoisted from the [`Dependency`] children of
/// [`Package`], keyed by the prefix documents use.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	Component,
	Reflect,
	LexiconDef,
	Serialize,
	Deserialize,
)]
#[reflect(Component, Default, LexiconDef, Serialize, Deserialize)]
pub struct Dependencies(pub BTreeMap<PackageName, Dependency>);

/// What a [`Dependency`] resolved to: a release for a version requirement, a
/// branch's package record for `#branch`, or for a path within one source
/// directory that package's package record.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize,
)]
#[reflect(Default, Serialize, Deserialize)]
pub struct ResolvedDependency {
	/// The account it lives in.
	pub did: Did,
	/// The version it was at.
	pub version: Version,
	/// The record as it was: its uri says by its form whether it names a
	/// space, and a cid that no longer matches refuses the resolve.
	pub record: StrongRef,
}

/// A prelude, a child of [`Package`]: a dependency's `.` export appears
/// unprefixed.
///
/// ```html
/// <Import name="." from="widgets"/>
/// ```
#[derive(
	Debug, Clone, PartialEq, Eq, Component, Reflect, Serialize, Deserialize,
)]
#[reflect(Component, SceneMeta, Serialize, Deserialize)]
pub struct Import {
	/// The export imported, `.` for the whole namespace.
	pub name: SmolStr,
	/// The dependency's key.
	pub from: PackageName,
}

impl SceneMeta for Import {
	fn apply(cx: &mut SceneMetaContext) -> Result {
		let parent = Package::parent_of(cx, "Import")?;
		let import = cx.get::<Self>()?.clone();
		Package::hoisted(cx, parent)?.imports.push(import);
		Ok(())
	}
}

/// One binary the package ships, a child of [`Package`], hoisted into
/// [`ExportRuntimes`]; its files are entries in the record's `Files`.
#[derive(
	Debug, Clone, PartialEq, Eq, Component, Reflect, Serialize, Deserialize,
)]
#[reflect(Component, SceneMeta, Serialize, Deserialize)]
pub struct Runtime {
	/// The runtime's name, unique with its platform.
	pub name: RuntimeName,
	/// What it runs on and the files it needs.
	pub platform: Platform,
	/// The crates a document may verify against, the shape
	/// [`CrateRegistration`] carries.
	#[serde(default)]
	#[reflect(default)]
	pub crates: BTreeMap<SmolStr, RuntimeCrate>,
}

impl SceneMeta for Runtime {
	fn apply(cx: &mut SceneMetaContext) -> Result {
		Package::parent_of(cx, "Runtime")?;
		let runtime = cx.get::<Self>()?.clone();
		let error = cx.error(format!(
			"the runtime `{}` is declared twice for `{}`",
			runtime.name,
			runtime.platform.triple()
		));
		let mut runtimes = cx.meta_mut::<ExportRuntimes>();
		match runtimes.0.iter().any(|other| {
			other.name == runtime.name
				&& other.platform.triple() == runtime.platform.triple()
		}) {
			true => Err(error),
			false => {
				runtimes.0.push(runtime);
				Ok(())
			}
		}
	}
}

/// One crate of a [`Runtime`].
#[derive(
	Debug, Default, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize,
)]
#[reflect(Default, Serialize, Deserialize)]
pub struct RuntimeCrate {
	/// The crate's version.
	pub version: Version,
	/// Its enabled features.
	#[serde(default)]
	pub features: Vec<SmolStr>,
}

/// The files a platform needs, as paths into the record's `Files`. One
/// variant per target, so an arm64 and an x64 linux are two runtimes, as they
/// are two binaries.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub enum Platform {
	/// A browser: the wasm module and its js glue.
	Browser {
		/// The wasm module.
		wasm: RelPath,
		/// Its js glue.
		js: RelPath,
	},
	/// `x86_64-unknown-linux-gnu`.
	LinuxX64 {
		/// The binary.
		bin: RelPath,
	},
	/// `aarch64-unknown-linux-gnu`.
	LinuxArm64 {
		/// The binary.
		bin: RelPath,
	},
	/// `aarch64-apple-darwin`.
	MacosArm64 {
		/// The binary.
		bin: RelPath,
	},
	/// `xtensa-esp32-none-elf`.
	Esp32 {
		/// The binary.
		bin: RelPath,
	},
}

impl Platform {
	/// The target triple the variant builds for.
	pub fn triple(&self) -> &'static str {
		match self {
			Self::Browser { .. } => "wasm32-unknown-unknown",
			Self::LinuxX64 { .. } => "x86_64-unknown-linux-gnu",
			Self::LinuxArm64 { .. } => "aarch64-unknown-linux-gnu",
			Self::MacosArm64 { .. } => "aarch64-apple-darwin",
			Self::Esp32 { .. } => "xtensa-esp32-none-elf",
		}
	}
}

/// `org.beet.core#exportRuntimes`, hoisted from the [`Runtime`] children of
/// [`Package`]: every binary the package ships, unique by name and platform.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	Component,
	Reflect,
	LexiconDef,
	Serialize,
	Deserialize,
)]
#[reflect(Component, Default, LexiconDef, Serialize, Deserialize)]
pub struct ExportRuntimes(pub Vec<Runtime>);

use beet::prelude::*;

/// Compiles a package's branch into scene records and a package record, in
/// the package's space under the package store's `.beet/pds/`
/// ([`PackageCompiler`]). The entry is a document or the directory holding
/// it, relative to the package store, which the launch's `--package` selects,
/// and a second run over unchanged files writes nothing:
///
/// ```sh
/// beet compile site                    # site/main.bsx, its `main` branch
/// beet compile examples/beetmania --branch=dev
/// ```
#[action(route = "compile/*entry")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn Compile(
	/// The branch compiled, `main` when absent.
	#[param]
	branch: Option<BranchName>,
	cx: ActionContext<RequestParts>,
) -> Result<String> {
	let pds = CompileRun::pds(&cx, false).await?;
	CompileRun::compile(&cx, &pds, branch)
		.await?
		.to_string()
		.xok()
}

/// Freezes a package's branch as a release at its version, in the package's
/// space ([`PackageReleaser::release`]): compiled first, which writes
/// nothing when it is current, then its package record copied into a
/// release that is never rewritten, and `latest` moved when it is the newest
/// version with no prerelease. A release stays private until published:
///
/// ```sh
/// beet release site
/// ```
#[action(route = "release/*entry")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn Release(
	/// The branch released, `main` when absent.
	#[param]
	branch: Option<BranchName>,
	cx: ActionContext<RequestParts>,
) -> Result<String> {
	let pds = CompileRun::pds(&cx, false).await?;
	let compiled = CompileRun::compile(&cx, &pds, branch).await?;
	let released = PackageReleaser::new(pds)
		.release(&compiled.package_record)
		.await?;
	format!("{compiled}\n{released}").xok()
}

/// Releases a package's branch, then copies the release to the account's
/// public repo byte for byte with every scene and blob it names
/// ([`PackageReleaser::publish`]), where an anonymous runtime reads it.
/// `--as-home` makes it the account's home package; `--dry-run` reports what
/// would be written and writes nothing:
///
/// ```sh
/// beet publish site --as-home --dry-run
/// ```
#[action(route = "publish/*entry")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn Publish(
	/// The branch published, `main` when absent.
	#[param]
	branch: Option<BranchName>,
	/// Write the account's home record naming this package, replacing one
	/// naming another.
	#[param]
	as_home: bool,
	/// Report what would be written, writing nothing.
	#[param]
	dry_run: bool,
	cx: ActionContext<RequestParts>,
) -> Result<String> {
	let pds = CompileRun::pds(&cx, dry_run).await?;
	let compiled = CompileRun::compile(&cx, &pds, branch).await?;
	let releaser = PackageReleaser::new(pds);
	let released = releaser.release(&compiled.package_record).await?;
	let published = releaser.publish(&released.record, as_home).await?;
	format!("{compiled}\n{released}{published}").xok()
}

/// What the three verbs share: the package store's own PDS, and the compile
/// of the entry the request names.
struct CompileRun;

impl CompileRun {
	/// The PDS under the package store's `.beet/pds/`, a [`DryRunPds`] over
	/// it for a dry run.
	async fn pds(
		cx: &ActionContext<RequestParts>,
		dry_run: bool,
	) -> Result<Pds> {
		let source = cx.world().with(PackageStore::get).await?;
		let pds = PackageCompiler::local_pds(&source);
		match dry_run {
			true => pds.dry(),
			false => pds,
		}
		.xok()
	}

	/// Compile the entry the request names into `pds`.
	async fn compile(
		cx: &ActionContext<RequestParts>,
		pds: &Pds,
		branch: Option<BranchName>,
	) -> Result<CompileReport> {
		let entry = FwdPath::new(crate::commands::entry_arg(&cx.input)?)?;
		let (source, parsers) = cx
			.world()
			.with(|world: &mut World| -> Result<_> {
				(PackageStore::get(world)?, DocParsers::of(world)).xok()
			})
			.await?;
		let mut compiler = PackageCompiler::new(parsers, pds.clone());
		if let Some(branch) = branch {
			compiler = compiler.with_branch(branch);
		}
		let entry = compiler.entry_of(&source, &entry).await?;
		compiler.compile(&source, &entry).await
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A world serving the three verbs over a clean copy of the site, and
	/// the host to call them on.
	fn site_world() -> (World, Entity, TempDir) {
		let mut world = crate::commands::render_world();
		let site = crate::commands::clean_site();
		world.spawn((
			BlobStore::new(FsStore::new(AbsPath::new(site.path()).unwrap())),
			PackageStore,
		));
		let host = world
			.spawn((Router, children![Compile, Release, Publish]))
			.id();
		(world, host, site)
	}

	/// Run `args` on `host`, answering the text.
	async fn run(world: &mut World, host: Entity, args: &str) -> String {
		world
			.entity_mut(host)
			.call::<Request, Response>(
				Request::from_cli_args(CliArgs::parse(args))
					.with_header::<header::Accept>(vec![MediaType::Text]),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await
	}

	/// The verb compiles a package into the package store's PDS, and a
	/// second run writes nothing.
	#[beet::test]
	async fn compiles_a_package() {
		let (mut world, host, _site) = site_world();
		run(&mut world, host, "compile .")
			.await
			.xpect_contains("compiled ");
		run(&mut world, host, "compile .")
			.await
			.xpect_contains("0 written")
			.xpect_contains("0 blobs uploaded");
	}

	/// A release then a publish, each a second time unchanged, and a dry
	/// run wording what it would do.
	#[beet::test]
	async fn releases_and_publishes() {
		let (mut world, host, _site) = site_world();
		run(&mut world, host, "publish . --dry-run")
			.await
			.xpect_contains("would create");
		run(&mut world, host, "release .")
			.await
			.xpect_contains("created")
			.xpect_contains("/org.beet.release/");
		run(&mut world, host, "release .")
			.await
			.xnot()
			.xpect_contains("created");
		run(&mut world, host, "publish . --as-home")
			.await
			.xpect_contains("/org.beet.home/self");
		run(&mut world, host, "publish . --as-home")
			.await
			.xnot()
			.xpect_contains("created")
			.xnot()
			.xpect_contains("updated");
	}
}

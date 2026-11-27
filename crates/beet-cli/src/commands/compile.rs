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
	let entry = RelPath::new(crate::commands::entry_arg(&cx.input)?);
	let (source, parsers) = cx
		.world()
		.with(|world: &mut World| -> Result<_> {
			(PackageStore::get(world)?, DocParsers::of(world)).xok()
		})
		.await?;
	let mut compiler =
		PackageCompiler::new(parsers, PackageCompiler::local_pds(&source));
	if let Some(branch) = branch {
		compiler = compiler.with_branch(branch);
	}
	let entry = compiler.entry_of(&source, &entry).await?;
	compiler.compile(&source, &entry).await?.to_string().xok()
}

#[cfg(test)]
mod test {
	use super::*;

	/// The verb compiles a package into the package store's PDS, and a
	/// second run writes nothing.
	#[beet::test]
	async fn compiles_a_package() {
		let mut world = crate::commands::render_world();
		let site = crate::commands::clean_site();
		world.spawn((
			BlobStore::new(FsStore::new(AbsPath::new(site.path()).unwrap())),
			PackageStore,
		));
		let host = world.spawn((Router, children![Compile])).id();
		let run = async |world: &mut World| {
			world
				.entity_mut(host)
				.call::<Request, Response>(
					Request::from_cli_args(CliArgs::parse("compile ."))
						.with_header::<header::Accept>(vec![MediaType::Text]),
				)
				.await
				.unwrap()
				.unwrap_str()
				.await
		};
		run(&mut world).await.xpect_contains("compiled ");
		run(&mut world)
			.await
			.xpect_contains("0 written")
			.xpect_contains("0 blobs uploaded");
	}
}

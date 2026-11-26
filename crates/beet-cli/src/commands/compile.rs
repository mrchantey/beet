use beet::prelude::*;

/// Compiles a package's branch into scene records and a package record, in
/// the package's space under the home directory's `.beet/repo/`
/// ([`PackageCompiler`]). The home directory is where `beet` runs unless
/// `--home` names another, the entry a document or the directory holding it,
/// relative to the home directory, and a second run over unchanged files
/// writes nothing:
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
	branch: Option<String>,
	/// The home directory, whose `.beet/repo/` the compile writes to, the
	/// working directory when absent.
	#[param]
	home: Option<String>,
	cx: ActionContext<RequestParts>,
) -> Result<String> {
	let entry = RelPath::new(crate::commands::entry_arg(&cx.input)?);
	let home = BlobStore::from_uri(&StoreUri::parse(&format!(
		"fs:{}",
		home.as_deref().unwrap_or(".")
	))?)?;
	let parsers = cx
		.world()
		.with(|world: &mut World| DocParsers::of(world))
		.await;
	let mut compiler =
		PackageCompiler::new(parsers, PackageCompiler::local_repo(&home));
	if let Some(branch) = branch {
		compiler = compiler.with_branch(BranchName::parse(&branch)?);
	}
	let entry = compiler.entry_of(&home, &entry).await?;
	compiler.compile(&home, &entry).await?.to_string().xok()
}

#[cfg(test)]
mod test {
	use super::*;

	/// The verb compiles a package into the home directory's repo, and a
	/// second run writes nothing.
	#[beet::test]
	async fn compiles_a_package() {
		let mut world = crate::commands::render_world();
		let host = world.spawn((Router, children![Compile])).id();
		let site = crate::commands::clean_site();
		let run = async |world: &mut World| {
			world
				.entity_mut(host)
				.call::<Request, Response>(
					Request::from_cli_args(CliArgs::parse(&format!(
						"compile . --home={}",
						site.path()
					)))
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

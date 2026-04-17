//! Package step for Lambda deploy sequences.
//! Creates `lambda.zip` containing the built binary as `bootstrap`.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Packages the built binary into a `lambda.zip` for AWS Lambda deployment.
/// Reads [`CargoBuildCmd`] from an ancestor to find the binary path,
/// [`Stack`] to determine the tofu working directory
#[action]
#[derive(Default, Component)]
pub async fn PackageLambdaAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	// find binary path from CargoBuildCmd on ancestor
	let exe_path = cx
		.caller
		.with_state::<AncestorQuery<&CargoBuildCmd>, _>(|entity, query| {
			query.get(entity).map(|cmd| cmd.exe_path(None))
		})
		.await?;
	let exe_path = AbsPathBuf::new(exe_path)?;

	// find the tofu working directory from Stack on ancestor
	let work_dir = cx
		.caller
		.with_state::<AncestorQuery<&Stack>, _>(|entity, query| {
			query
				.get(entity)
				.map(|stack| stack.work_directory().into_abs())
		})
		.await?;

	let zip_path = work_dir.join("lambda.zip");
	info!("packaging {} -> {}", exe_path.display(), zip_path.display());

	// ensure work directory exists
	fs_ext::create_dir_all_async(&work_dir).await?;

	// copy the binary as 'bootstrap' in work dir
	let bootstrap_path = work_dir.join("bootstrap");
	let binary_bytes = fs_ext::read_async(&exe_path).await?;
	fs_ext::write_async(&bootstrap_path, &binary_bytes).await?;

	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		std::fs::set_permissions(
			bootstrap_path.as_path(),
			std::fs::Permissions::from_mode(0o755),
		)?;
	}

	// remove old zip
	if fs_ext::exists_async(&zip_path).await.unwrap_or(false) {
		fs_ext::remove_async(&zip_path).await?;
	}

	// create zip with bootstrap (flat, no directory structure)
	let zip_str = zip_path.display().to_string();
	let bootstrap_str = bootstrap_path.display().to_string();
	ChildProcess::new("zip")
		.with_args(&["-j", &zip_str, &bootstrap_str])
		.run_async()
		.await
		.map_err(|err| bevyhow!("failed to create lambda.zip: {err}"))?;

	// clean up
	fs_ext::remove_async(&bootstrap_path).await?;

	info!("lambda.zip created successfully");
	Pass(cx.input).xok()
}

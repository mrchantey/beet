//! What every Office reader's and writer's tests share: a document parsed
//! into a world of the formats, written back, and rendered as HTML or
//! markdown to compare.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// `bytes` parsed into a world of the formats and the default style rules,
/// and its root.
pub(crate) fn parse(bytes: &MediaBytes) -> Result<(World, Entity)> {
	parse_in(
		(TemplatePlugin, DocumentPlugin, OoxmlPlugin).into_world(),
		bytes,
	)
}

/// `bytes` parsed into `world`, and its root.
pub(crate) fn parse_in(
	mut world: World,
	bytes: &MediaBytes,
) -> Result<(World, Entity)> {
	let root = world.spawn_empty().id();
	DocParsers::parse(ParseContext::new(&mut world.entity_mut(root), bytes))?;
	(world, root).xok()
}

/// `root` written as `media_type` for `request`.
pub(crate) fn write(
	world: &mut World,
	root: Entity,
	media_type: MediaType,
	request: RequestParts,
) -> Result<MediaBytes> {
	DocRenderers::render(world, root, &request.with_accept(media_type))?.xok()
}

/// The part at `path` of a written package as text.
pub(crate) fn part(bytes: &MediaBytes, path: &str) -> String {
	OpcPackage::read(bytes.bytes())
		.unwrap()
		.part(path)
		.map(|part| String::from_utf8(part.to_vec()).unwrap())
		.unwrap_or_else(|| panic!("the package holds no `{path}`"))
}

/// The tree under `root` as HTML.
pub(crate) fn html(world: &mut World, root: Entity) -> String {
	HtmlRenderer::new()
		.render(&mut RenderContext::new(
			world,
			root,
			&RequestParts::default(),
		))
		.unwrap()
		.to_string()
}

/// Whether LibreOffice is on this machine, a test converting through it
/// skipping where it is not.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn has_soffice() -> bool {
	std::process::Command::new("soffice")
		.arg("--version")
		.output()
		.is_ok()
}

/// `bytes` written as `name` in a fresh directory and converted by
/// LibreOffice headless to `format`, its own profile kept in that
/// directory, answering the converted file's text.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn soffice_convert(
	bytes: &MediaBytes,
	name: &str,
	format: &str,
) -> String {
	let dir = TempDir::new().unwrap();
	let path: &std::path::Path = dir.as_ref();
	let file = path.join(name);
	fs_ext::write(&file, bytes.bytes()).unwrap();
	let output = std::process::Command::new("soffice")
		.arg(format!(
			"-env:UserInstallation=file://{}",
			path.join("profile").display()
		))
		.args(["--headless", "--convert-to", format, "--outdir"])
		.arg(path)
		.arg(&file)
		.output()
		.unwrap();
	assert!(
		output.status.success(),
		"soffice failed: {}",
		String::from_utf8_lossy(&output.stderr)
	);
	let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
	let extension = format.split(':').next().unwrap_or(format);
	fs_ext::read_to_string(path.join(format!("{stem}.{extension}"))).unwrap()
}

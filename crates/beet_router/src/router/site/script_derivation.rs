//! Files a script derives from resources of the world, ie a logo drawn in the
//! brand colours.
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use std::collections::BTreeMap;

/// `<ScriptDerivation src="scripts/logo.js" call="BeetLogo.files"
/// dir="assets/logo" resources={["ThemeTones"]}/>`: the text files a script
/// answers, derived into `dir` in the store this entity resolves by ancestry.
///
/// `src` is a classic script, no `import` or `export`, so a page can load the
/// very same file with `<script src>` and draw live what is derived here. Run
/// as the body of the derivation, it is followed by `return <call>(input)`,
/// the call answering an object of file path, relative to `dir`, to text. Its
/// `input` holds each
/// of `resources` by the name it is declared by, serialized as the world holds
/// it ([`Derivation::read_resources`]), ie `input.ThemeTones.primary[40]`. It
/// runs pure, with no `world` bridge, since a derived file depends on what its
/// provenance names and nothing else.
///
/// The declaration is its own producing action. Its [`Derivation`] is
/// `<dir>/<src stem>.derived.json`, listing every file written, its input the
/// script and its resources the ones handed to it, read as the file's
/// freshness is checked, so editing the script or a change to a resource
/// regenerates the files at the next `<Derive/>`. Declare it before a
/// declaration reading its files (an [`IconSet`](crate::prelude::IconSet)
/// drawn from the svg), since `<Derive/>` ensures in document order.
#[action]
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::component_hook(ScriptDerivation::declare_derivation))]
pub async fn ScriptDerivation(
	/// The script, a classic script answering `call`.
	#[field]
	src: FwdPath,
	/// The expression the script answers with, called with its input.
	#[field]
	call: SmolStr,
	/// The directory the files are written into.
	#[field]
	dir: FwdPath,
	/// The resources the script is handed, each by a short or full type path.
	#[field]
	resources: Vec<ReflectName>,
	cx: ActionContext<BlobStore>,
) -> Result<EntityBody> {
	let declaration = ScriptDerivation {
		src,
		call,
		dir,
		resources,
	};
	let source = cx.input.get(&declaration.src).await?;
	let derivation = declaration.derivation()?;
	let input = cx
		.world()
		.with(move |world: &mut World| derivation.read_resources(world))
		.await?
		.into_iter()
		.map(|(name, value)| (SmolStr::from(name), value))
		.collect::<Map>();
	let body = format!(
		"{{\n{}\nreturn {}(input);\n}}",
		str::from_utf8(&source)?,
		declaration.call
	);
	let config = ScriptConfig {
		world: false,
		..default()
	};
	let files = Script::<Value, BTreeMap<FwdPath, String>>::new(body)
		.run(Value::Map(input), cx.world().clone(), &config)
		.await?;
	let mut outputs = DerivationOutputs::default();
	for (path, text) in files {
		outputs
			.write(&cx.input, declaration.output(&path)?, text.into_bytes())
			.await?;
	}
	EntityBody::default()
		.with_typed(&Provenance::new([ProvenanceSource::file(
			declaration.src.clone(),
			&source,
		)]))?
		.with_typed(&outputs)
}

impl ScriptDerivation {
	/// The [`Derivation`] this declaration declares.
	pub fn derivation(&self) -> Result<Derivation> {
		let stem = self.src.as_smol_path().file_stem().ok_or_else(|| {
			bevyhow!("script `{}` has no file name", self.src)
		})?;
		self.resources
			.iter()
			.fold(
				Derivation::new(self.dir.try_join(format!("{stem}.derived.json"))?)
					.with_input(self.src.clone()),
				|derivation, resource| {
					derivation.with_resource(resource.clone())
				},
			)
			.xok()
	}

	/// The store path of the file at `path` in `dir` the script answered. A
	/// [`FwdPath`] cannot climb out of `dir`, and an empty one names no file.
	fn output(&self, path: &FwdPath) -> Result<FwdPath> {
		if path.as_str().is_empty() {
			bevybail!(
				"script `{}` answered a file with no path within `{}`",
				self.src,
				self.dir
			);
		}
		self.dir.join(path).xok()
	}

	/// The hook landing this declaration's [`Derivation`] on its entity.
	fn declare_derivation(&self) -> impl FnOnce(&mut EntityCommands) + use<> {
		let derivation = self.derivation();
		move |entity: &mut EntityCommands| match derivation {
			Ok(derivation) => {
				entity.insert(derivation);
			}
			Err(error) => entity.commands().queue(move |world: &mut World| {
				world.handle_command_error::<Self>(error)
			}),
		}
	}
}

#[cfg(all(test, feature = "quickjs"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	const SRC: FwdPath = fwd_path!("scripts/swatch.js");

	/// A world with the theme's tones, declaring a derivation of `script` into
	/// `out` handed them, in a temp store.
	fn setup(script: &str) -> (World, Entity, BlobStore) {
		let mut world =
			(AsyncPlugin, material::MaterialStylePlugin).into_world();
		world.insert_resource(Theme {
			primary: Some(Color::srgb_u8(0x00, 0x6c, 0x4f)),
			..default()
		});
		world.flush();
		let store = BlobStore::temp();
		let entity = world
			.spawn(ScriptDerivation {
				src: SRC,
				call: "Swatch.files".into(),
				dir: fwd_path!("out"),
				resources: vec!["ThemeTones".into()],
			})
			.flush();
		let inserted = store.clone();
		let script = script.to_string();
		async_ext::block_on(async move {
			inserted.insert(&SRC, script).await
		})
		.unwrap();
		(world, entity, store)
	}

	async fn ensure(
		world: &mut World,
		entity: Entity,
		store: &BlobStore,
	) -> Result<Freshness> {
		let store = store.clone();
		world
			.run_async_then(async move |world| {
				Derivation::ensure(&world, entity, &store).await
			})
			.await
	}

	#[beet_core::test]
	async fn derives_files_in_the_theme() {
		let (mut world, entity, store) = setup(
			"const Swatch = { files: (input) => ({ 'swatch.svg': `<rect fill=\"${input.ThemeTones.primary[40]}\"/>` }) };",
		);
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Missing);
		str::from_utf8(&store.get(&fwd_path!("out/swatch.svg")).await.unwrap())
			.unwrap()
			.xpect_eq("<rect fill=\"#006C4F\"/>");
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Fresh);
		// a new theme is new tones, read as the freshness is checked
		world.insert_resource(Theme::default());
		world.flush();
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Revalued("ThemeTones".into()));
	}

	#[beet_core::test]
	async fn writes_nested_files() {
		let (mut world, entity, store) =
			setup("const Swatch = { files: () => ({ 'icons/a.svg': '' }) };");
		ensure(&mut world, entity, &store).await.unwrap();
		store
			.exists(&fwd_path!("out/icons/a.svg"))
			.await
			.unwrap()
			.xpect_true();
	}

	#[beet_core::test]
	async fn refuses_a_path_climbing_out() {
		let (mut world, entity, store) =
			setup("const Swatch = { files: () => ({ '../escape.svg': '' }) };");
		ensure(&mut world, entity, &store)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("climbs out");
		store
			.exists(&fwd_path!("escape.svg"))
			.await
			.unwrap()
			.xpect_false();
	}

	#[beet_core::test]
	async fn refuses_a_file_with_no_path() {
		let (mut world, entity, store) =
			setup("const Swatch = { files: () => ({ '': '' }) };");
		ensure(&mut world, entity, &store)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("a file with no path");
	}
}

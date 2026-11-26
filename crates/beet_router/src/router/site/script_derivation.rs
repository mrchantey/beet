//! Files a script derives from the brand colours, ie a logo drawn in svg.
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;
use serde::Serialize;
use std::collections::BTreeMap;

/// `<ScriptDerivation src="scripts/logo.js" call="BeetLogo.files"
/// dir="assets/logo"/>`: the text files a script answers, derived into `dir`
/// in the store this entity resolves by ancestry.
///
/// `src` is a classic script, no `import` or `export`, so a page can load the
/// very same file with `<script src>` and draw live what is derived here. Run
/// as the body of the derivation, it is followed by `return <call>(input)`,
/// the call answering an object of file name to text. Its `input` is
/// [`ScriptDerivationInput`], the active [`Theme`]'s tones, so a script draws
/// in the brand colours without carrying colour maths. It runs pure, with no
/// `world` bridge, since a derived file depends on what its provenance names
/// and nothing else.
///
/// The declaration is its own producing action. Its [`Derivation`] is
/// `<dir>/<src stem>.derived.json`, listing every file written, its input the
/// script and its value the tones, so editing the script or the theme
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
	src: RelPath,
	/// The expression the script answers with, called with its input.
	#[field]
	call: SmolStr,
	/// The directory the files are written into.
	#[field]
	dir: RelPath,
	cx: ActionContext<BlobStore>,
) -> Result<EntityBody> {
	let declaration = ScriptDerivation { src, call, dir };
	let source = cx.input.get(&declaration.src).await?;
	let input = cx
		.world()
		.with(|world: &mut World| ScriptDerivationInput::new(world))
		.await;
	let body = format!(
		"{{\n{}\nreturn {}(input);\n}}",
		str::from_utf8(&source)?,
		declaration.call
	);
	let config = ScriptConfig {
		world: false,
		..default()
	};
	let files = Script::<ScriptDerivationInput, BTreeMap<String, String>>::new(
		body,
	)
	.run(input, cx.world().clone(), &config)
	.await?;
	let mut outputs = DerivationOutputs::default();
	for (name, text) in files {
		outputs
			.write(&cx.input, declaration.output(&name)?, text.into_bytes())
			.await?;
	}
	EntityBody::default()
		.with_typed(&Provenance::new([ProvenanceSource::file(
			declaration.src.clone(),
			&source,
		)]))?
		.with_typed(&outputs)
}

/// What a [`ScriptDerivation`]'s script is handed as `input`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScriptDerivationInput {
	/// Every tone of the active [`Theme`]'s palettes, `tones.primary[40]`.
	pub tones: ThemeTones,
}

impl ScriptDerivationInput {
	/// The input in `world`, its [`Theme`] or the default.
	pub fn new(world: &World) -> Self {
		let theme = world.get_resource::<Theme>().cloned().unwrap_or_default();
		Self {
			tones: ThemePalettes::new(&theme).tones(),
		}
	}
}

impl ScriptDerivation {
	/// The value a derivation names the input by.
	const VALUE: &str = "input";

	/// The [`Derivation`] this declaration declares, computed from `input`.
	pub fn derivation(&self, input: &ScriptDerivationInput) -> Result<Derivation> {
		let stem = self
			.src
			.as_smol_path()
			.file_stem()
			.ok_or_else(|| bevyhow!("script `{}` has no file name", self.src))?;
		Derivation::new(self.dir.join(format!("{stem}.derived.json")))
			.with_input(self.src.clone())
			.with_value(ProvenanceValue::new(
				Self::VALUE,
				&MediaType::Json.serialize(input)?,
			))
			.xok()
	}

	/// The store path of the file `name` the script answered, refusing a name
	/// reaching outside `dir`.
	fn output(&self, name: &str) -> Result<RelPath> {
		if name.is_empty()
			|| name.starts_with('.')
			|| name.contains(['/', '\\'])
		{
			bevybail!(
				"script `{}` answered the file `{name}`: a file name is one plain path segment",
				self.src
			);
		}
		self.dir.join(name).xok()
	}

	/// The hook landing this declaration's [`Derivation`] on its entity,
	/// computed from the theme at hand; [`Self::refresh`] recomputes it when
	/// the theme lands.
	fn declare_derivation(&self) -> impl FnOnce(&mut EntityCommands) + use<> {
		let declaration = self.clone();
		move |entity: &mut EntityCommands| {
			let id = entity.id();
			entity.commands().queue(move |world: &mut World| {
				Self::refresh_one(world, id, &declaration)
			});
		}
	}

	/// Recompute every declaration's [`Derivation`] from the world's theme.
	fn refresh(world: &mut World) -> Result {
		let declarations = world
			.query::<(Entity, &Self)>()
			.iter(world)
			.map(|(entity, declaration)| (entity, declaration.clone()))
			.collect::<Vec<_>>();
		for (entity, declaration) in declarations {
			Self::refresh_one(world, entity, &declaration)?;
		}
		Ok(())
	}

	fn refresh_one(world: &mut World, entity: Entity, declaration: &Self) -> Result {
		let derivation =
			declaration.derivation(&ScriptDerivationInput::new(world))?;
		if let Ok(mut entity) = world.get_entity_mut(entity) {
			entity.insert(derivation);
		}
		Ok(())
	}
}

impl TypeInstaller for ScriptDerivation {
	/// Registers the declaration and recomputes each one's input when the
	/// theme lands.
	fn install(app: &mut App) {
		app.register_type::<Self>().add_observer(
			|_ev: On<Insert<Theme>>, mut commands: Commands| {
				commands.queue(ScriptDerivation::refresh);
			},
		);
	}
}

#[cfg(all(test, feature = "quickjs"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	const SRC: &str = "scripts/swatch.js";

	/// A world declaring a derivation of `script` into `out`, in a temp store.
	fn setup(script: &str) -> (World, Entity, BlobStore) {
		let mut world = (AsyncPlugin, ScriptDerivation::install).into_world();
		world.insert_resource(Theme {
			primary: Some(Color::srgb_u8(0x00, 0x6c, 0x4f)),
			..default()
		});
		let store = BlobStore::temp();
		let entity = world
			.spawn(ScriptDerivation {
				src: SRC.into(),
				call: "Swatch.files".into(),
				dir: "out".into(),
			})
			.flush();
		let inserted = store.clone();
		let script = script.to_string();
		async_ext::block_on(async move {
			inserted.insert(&RelPath::new(SRC), script).await
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
			"const Swatch = { files: (input) => ({ 'swatch.svg': `<rect fill=\"${input.tones.primary[40]}\"/>` }) };",
		);
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Missing);
		str::from_utf8(&store.get(&RelPath::new("out/swatch.svg")).await.unwrap())
			.unwrap()
			.xpect_eq("<rect fill=\"#006C4F\"/>");
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.xpect_eq(Freshness::Fresh);
		// a new theme is a new input
		world.insert_resource(Theme::default());
		world.flush();
		ensure(&mut world, entity, &store)
			.await
			.unwrap()
			.to_string()
			.xpect_contains("input");
	}

	#[beet_core::test]
	async fn refuses_a_path_outside_its_dir() {
		let (mut world, entity, store) =
			setup("const Swatch = { files: () => ({ '../escape.svg': '' }) };");
		ensure(&mut world, entity, &store)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("one plain path segment");
	}
}

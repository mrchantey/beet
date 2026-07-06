//! Scene rotation for the perceive-act demo. The agent is one continuous creature
//! that rotates through scenes discovered in the blob store, each a directory with a
//! `main.bsx` character prompt and an `images/` set it can display. Rotation is an
//! in-place content swap: the fixed system prompt, socket server, capability routes
//! and head connection all survive, and each new character is *appended* as a user
//! turn (never mutating or clearing the window), so the creature carries its memory
//! across incarnations.
//!
//! [`maybe_rotate_scene`] runs at the start of each camera turn ([`PostPhoto`]): it
//! discovers the catalog on the first run and, every `every_cycles`, advances to the
//! next scene. The chosen scene's images populate a [`StringEnumOptions`] (via
//! [`sync_image_options`]) so the model's `respond-multi-modal` `image` field is
//! constrained to the scene's titles, and [`RespondMultiModalAction`] maps the chosen
//! title to its url.
use super::*;
use crate::beet::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Config + discovered catalog for scene rotation, spawned on the agent root.
///
/// Requires (and so auto-inserts) the runtime [`ActiveScene`] + [`SceneCatalog`]
/// state, so authoring just `{SceneRotation::default()}` (or with overrides) is
/// enough. Resolved by descendants via `AncestorQuery`, so multiple agents in one
/// world each rotate their own scenes from their own store.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ActiveScene, SceneCatalog)]
pub struct SceneRotation {
	/// The scenes directory in the blob store.
	pub dir: SmolPath,
	/// Perceive-act cycles between rotations.
	pub every_cycles: u32,
	/// Sequential vs random (shuffle-bag) order.
	pub order: SceneOrder,
	/// The scene to start on; falls back to the first discovered if absent/unknown.
	pub initial: Option<SmolStr>,
	/// Shown when a scene has no `images/` and when the model picks an unknown title.
	pub fallback_image: SmolPath,
}

impl Default for SceneRotation {
	fn default() -> Self {
		Self {
			dir: SmolPath::from("assets/extra/perceive-act"),
			every_cycles: 8,
			order: SceneOrder::default(),
			initial: Some(SmolStr::new("explorer")),
			fallback_image: SmolPath::from(
				"assets/extra/perceive-act/explorer/images/joy.png",
			),
		}
	}
}

/// The order [`SceneRotation`] visits scenes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[reflect(Default)]
pub enum SceneOrder {
	/// In discovered (sorted) order, wrapping.
	#[default]
	Sequential,
	/// Randomly, without repeating until every scene has shown (a shuffle-bag).
	Random,
}

/// The discovered scene catalog, filled lazily on the first [`maybe_rotate_scene`] run.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct SceneCatalog {
	/// Scene names (dirs under [`SceneRotation::dir`] holding a `main.bsx`).
	pub scenes: Vec<SmolStr>,
	/// Whether discovery has run.
	pub discovered: bool,
}

/// The active scene's state, updated each rotation.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct ActiveScene {
	/// The current scene name, empty before the first rotation.
	pub name: SmolStr,
	/// The displayable images the model may choose: title (file stem) + resolved url.
	pub images: Vec<SceneImage>,
	/// Scene names shown this round, for the random shuffle-bag.
	pub visited: Vec<SmolStr>,
	/// Url shown on an empty scene or an unknown model choice.
	pub fallback_url: SmolStr,
}

impl ActiveScene {
	/// The url for a chosen image `title`, or the fallback url if unknown.
	pub fn resolve_url(&self, title: &str) -> SmolStr {
		self.images
			.iter()
			.find(|image| image.title == title)
			.map(|image| image.url.clone())
			.unwrap_or_else(|| self.fallback_url.clone())
	}
	/// The image titles offered to the model.
	pub fn titles(&self) -> Vec<SmolStr> {
		self.images.iter().map(|image| image.title.clone()).collect()
	}
}

/// One displayable image: the `title` the model picks by (the file stem) and the
/// `url` the head renders.
#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub struct SceneImage {
	/// The file stem, eg `joy`, the model chooses by.
	pub title: SmolStr,
	/// The resolved url, eg `/assets/extra/perceive-act/explorer/images/joy.png`.
	pub url: SmolStr,
}

/// The character prompt of a scene, the single root element of its `main.bsx`:
/// `<ScenePrompt text="You are an intrepid explorer..."/>`. Loaded at rotation and
/// appended to the running thread as a user turn.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct ScenePrompt {
	/// The character prompt text.
	pub text: String,
}

/// State gathered in one world access before the async scene load.
struct Gathered {
	store: BlobStore,
	rotation: SceneRotation,
	discovered: bool,
	scenes: Vec<SmolStr>,
	active_name: SmolStr,
	visited: Vec<SmolStr>,
	cycle: u64,
	seed: u64,
}

/// The scene to apply next and the shuffle-bag state after applying it.
struct Plan {
	name: SmolStr,
	visited: Vec<SmolStr>,
}

/// Discover the scene catalog on the first call, then every
/// [`SceneRotation::every_cycles`] advance to the next scene, applying it in place.
/// Called at the start of the camera's turn ([`PostPhoto`](super::PostPhoto)), so a
/// fresh character is appended as a user turn just before the fresh photo.
pub(crate) async fn maybe_rotate_scene(caller: &AsyncEntity) -> Result {
	// gather config + rotation state (and one random seed) in a single world access.
	// no `SceneRotation` ancestor means rotation is not configured (eg a standalone
	// `PostPhoto`), so skip silently.
	let Some(mut gathered) = caller
		.with_state::<(
			AncestorQuery<(&SceneRotation, &SceneCatalog, &ActiveScene)>,
			AncestorQuery<&BlobStore>,
			Option<Res<CycleClock>>,
			Option<ResMut<RandomSource>>,
		), _>(
			|entity, (config, stores, clock, mut rng)| -> Result<Option<Gathered>> {
				let Ok((rotation, catalog, active)) = config.get(entity) else {
					return Ok(None);
				};
				Ok(Some(Gathered {
					store: stores.get(entity)?.clone(),
					rotation: rotation.clone(),
					discovered: catalog.discovered,
					scenes: catalog.scenes.clone(),
					active_name: active.name.clone(),
					visited: active.visited.clone(),
					cycle: clock.map(|clock| clock.cycle).unwrap_or(0),
					seed: rng.as_mut().map(|rng| rng.random()).unwrap_or(0),
				}))
			},
		)
		.await??
	else {
		return Ok(());
	};

	// discover the catalog on the first run, writing it back for later cycles.
	if !gathered.discovered {
		gathered.scenes =
			discover_scenes(&gathered.store, &gathered.rotation.dir).await?;
		let scenes = gathered.scenes.clone();
		caller
			.with_state::<AncestorQuery<&mut SceneCatalog>, _>(
				move |entity, mut catalogs| -> Result {
					let mut catalog = catalogs.get_mut(entity)?;
					catalog.scenes = scenes;
					catalog.discovered = true;
					Ok(())
				},
			)
			.await??;
		if gathered.scenes.is_empty() {
			warn!("no perceive-act scenes found in {}", gathered.rotation.dir);
		}
	}

	// decide the scene to apply this cycle, if any, then load + apply it in place.
	let Some(plan) = plan_rotation(&gathered) else {
		return Ok(());
	};
	if plan.name != gathered.active_name {
		apply_scene(caller, &gathered, &plan).await?;
	}
	Ok(())
}

/// Decide the scene to apply: the initial scene on the first run, or the next scene
/// on a rotation boundary, else `None`. Pure, so the rotation policy is testable.
fn plan_rotation(gathered: &Gathered) -> Option<Plan> {
	let Gathered {
		rotation,
		scenes,
		active_name,
		visited,
		cycle,
		seed,
		..
	} = gathered;
	if scenes.is_empty() {
		return None;
	}
	// first run: the configured initial scene, or the first discovered.
	if active_name.is_empty() {
		let name = rotation
			.initial
			.clone()
			.filter(|initial| scenes.contains(initial))
			.or_else(|| scenes.first().cloned())?;
		return Some(Plan {
			visited: vec![name.clone()],
			name,
		});
	}
	// otherwise only rotate on a cycle boundary.
	let due = rotation.every_cycles > 0
		&& *cycle > 0
		&& cycle % rotation.every_cycles as u64 == 0;
	if !due {
		return None;
	}
	let next = match rotation.order {
		SceneOrder::Sequential => {
			let index = scenes
				.iter()
				.position(|scene| scene == active_name)
				.unwrap_or(0);
			scenes[(index + 1) % scenes.len()].clone()
		}
		SceneOrder::Random => {
			// draw from the scenes not yet shown this round (a shuffle-bag),
			// refilling once every scene but the current has shown.
			let mut bag: Vec<SmolStr> = scenes
				.iter()
				.filter(|&scene| !visited.contains(scene) && scene != active_name)
				.cloned()
				.collect();
			if bag.is_empty() {
				bag = scenes
					.iter()
					.filter(|&scene| scene != active_name)
					.cloned()
					.collect();
			}
			bag.get(*seed as usize % bag.len().max(1))?.clone()
		}
	};
	// record the pick in the bag, resetting once the round is complete.
	let mut visited =
		if visited.len() >= scenes.len() { Vec::new() } else { visited.clone() };
	if !visited.contains(&next) {
		visited.push(next.clone());
	}
	Some(Plan { name: next, visited })
}

/// Load a scene's prompt + images and apply it in place: append the character prompt
/// as a user turn and update the [`ActiveScene`].
async fn apply_scene(
	caller: &AsyncEntity,
	gathered: &Gathered,
	plan: &Plan,
) -> Result {
	let SceneRotation { dir, fallback_image, .. } = &gathered.rotation;
	// read the scene's `main.bsx` and extract its `<ScenePrompt>` text.
	let main_path = SmolPath::from(format!("{dir}/{}/main.bsx", plan.name));
	let source = String::from_utf8(gathered.store.get(&main_path).await?.to_vec())?;
	let prompt = caller
		.world()
		.with(move |world| extract_scene_prompt(world, &source))
		.await?;
	// list the scene's displayable images, falling back to the configured image.
	let images =
		list_scene_images(&gathered.store, dir, &plan.name, fallback_image).await?;
	let fallback_url = to_url(fallback_image);
	let name = plan.name.clone();
	let visited = plan.visited.clone();
	// append the character prompt as a user turn and update the active scene.
	caller
		.with_state::<(ThreadWindowQuery, AncestorQuery<&mut ActiveScene>), _>(
			move |entity, (mut windows, mut actives)| -> Result {
				// author the scene prompt as the camera actor this runs as (user role).
				let thread_id = windows.thread_id(entity)?;
				let author = windows.actor_id(entity)?;
				windows.window_mut(entity)?.upsert_post(AgentPost::new_text(
					author,
					thread_id,
					prompt,
					PostStatus::Completed,
				));
				let mut active = actives.get_mut(entity)?;
				active.name = name;
				active.images = images;
				active.fallback_url = fallback_url;
				active.visited = visited;
				Ok(())
			},
		)
		.await??;
	info!("scene -> {}", plan.name);
	Ok(())
}

/// Parse a scene's `main.bsx` (a single `<ScenePrompt text=..>` root) and return its
/// text, using a throwaway entity so nothing lingers in the world.
fn extract_scene_prompt(world: &mut World, source: &str) -> Result<String> {
	let entry = BsxTemplate::parse_entry(world, source)?;
	let root = entry.spawn(world)?;
	let text = world
		.get::<ScenePrompt>(root)
		.map(|prompt| prompt.text.clone())
		.ok_or_else(|| {
			bevyhow!("a scene main.bsx must be a single <ScenePrompt text=..> element")
		});
	world.entity_mut(root).despawn();
	text
}

/// Discover scene names: list the rotation dir and keep each first path segment
/// that has a `main.bsx`.
async fn discover_scenes(
	store: &BlobStore,
	dir: &SmolPath,
) -> Result<Vec<SmolStr>> {
	let mut scenes: Vec<SmolStr> = store
		.with_subdir(dir.clone())
		.list()
		.await
		.unwrap_or_default()
		.iter()
		.filter_map(|path| {
			let path = path.to_string();
			let (name, rest) = path.split_once('/')?;
			(rest == "main.bsx").then(|| SmolStr::from(name))
		})
		.collect();
	scenes.sort();
	scenes.dedup();
	Ok(scenes)
}

/// List a scene's `images/`, mapping each image file to a [`SceneImage`]. An empty or
/// missing dir yields a single option from the configured fallback image.
async fn list_scene_images(
	store: &BlobStore,
	dir: &SmolPath,
	name: &str,
	fallback_image: &SmolPath,
) -> Result<Vec<SceneImage>> {
	let images_dir = SmolPath::from(format!("{dir}/{name}/images"));
	let mut files =
		store.with_subdir(images_dir).list().await.unwrap_or_default();
	files.sort();
	let images: Vec<SceneImage> = files
		.iter()
		.map(|file| file.to_string())
		.filter(|file| is_image_file(file))
		.map(|file| SceneImage {
			title: SmolStr::from(image_title(&file)),
			url: to_url(&SmolPath::from(format!("{dir}/{name}/images/{file}"))),
		})
		.collect();
	if images.is_empty() {
		Ok(vec![SceneImage {
			title: SmolStr::from(image_title(&fallback_image.to_string())),
			url: to_url(fallback_image),
		}])
	} else {
		Ok(images)
	}
}

/// Keep every `image` [`StringEnumOptions`] in sync with the nearest [`ActiveScene`]'s
/// titles, so the model's `respond-multi-modal` choices track the active scene. Writing
/// only on a diff, so the [`ToolDefinition`] is repatched only when the scene changes.
pub(crate) fn sync_image_options(
	mut tools: Query<(Entity, &mut StringEnumOptions)>,
	scenes: AncestorQuery<&ActiveScene>,
) {
	for (entity, mut options) in &mut tools {
		if options.field != "image" {
			continue;
		}
		let Ok(scene) = scenes.get(entity) else {
			continue;
		};
		let titles = scene.titles();
		if options.options != titles {
			options.options = titles;
		}
	}
}

/// Whether a file name has an image extension.
fn is_image_file(file: &str) -> bool {
	matches!(
		file.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()),
		Some(ext) if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp")
	)
}

/// The title (file stem) of an image path, eg `a/b/joy.png` -> `joy`.
fn image_title(file: &str) -> &str {
	let file = file.rsplit('/').next().unwrap_or(file);
	file.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(file)
}

/// A store-relative path to a served url (a leading slash), eg
/// `assets/x/joy.png` -> `/assets/x/joy.png`.
fn to_url(path: &SmolPath) -> SmolStr { SmolStr::from(format!("/{path}")) }

#[cfg(test)]
mod test {
	use super::*;

	fn gathered(
		order: SceneOrder,
		scenes: &[&str],
		active: &str,
		visited: &[&str],
		cycle: u64,
		seed: u64,
	) -> Gathered {
		Gathered {
			store: BlobStore::temp(),
			rotation: SceneRotation { order, ..default() },
			discovered: true,
			scenes: scenes.iter().map(|scene| SmolStr::new(scene)).collect(),
			active_name: SmolStr::new(active),
			visited: visited.iter().map(|scene| SmolStr::new(scene)).collect(),
			cycle,
			seed,
		}
	}

	/// The first run applies the configured initial scene.
	#[beet_core::test]
	fn first_run_uses_initial() {
		let plan = plan_rotation(&gathered(
			SceneOrder::Sequential,
			&["dopey", "explorer", "grumpy"],
			"",
			&[],
			0,
			0,
		));
		plan.map(|plan| plan.name).xpect_eq(Some(SmolStr::new("explorer")));
	}

	/// Sequential rotation advances on a cycle boundary and wraps.
	#[beet_core::test]
	fn sequential_advances_on_boundary() {
		let scenes = ["dopey", "explorer", "grumpy"];
		// not a boundary: no rotation.
		plan_rotation(&gathered(SceneOrder::Sequential, &scenes, "dopey", &[], 3, 0))
			.map(|plan| plan.name)
			.xpect_eq(None);
		// boundary: advance to the next scene.
		plan_rotation(&gathered(SceneOrder::Sequential, &scenes, "dopey", &[], 8, 0))
			.map(|plan| plan.name)
			.xpect_eq(Some(SmolStr::new("explorer")));
		// wraps at the end.
		plan_rotation(&gathered(SceneOrder::Sequential, &scenes, "grumpy", &[], 8, 0))
			.map(|plan| plan.name)
			.xpect_eq(Some(SmolStr::new("dopey")));
	}

	/// Random rotation never repeats the current scene and covers the bag before
	/// repeating.
	#[beet_core::test]
	fn random_uses_shuffle_bag() {
		let scenes = ["dopey", "explorer", "grumpy"];
		// with two already visited, the only unshown non-current scene is picked.
		let plan = plan_rotation(&gathered(
			SceneOrder::Random,
			&scenes,
			"dopey",
			&["dopey", "explorer"],
			8,
			0,
		))
		.unwrap();
		// the only unshown, non-current scene.
		plan.name.xpect_eq(SmolStr::new("grumpy"));
	}

	/// Discovery lists only scene dirs that have a `main.bsx`; image listing maps files
	/// to titles + served urls, and an empty scene falls back to the configured image.
	#[beet_core::test]
	async fn discovers_scenes_and_images() {
		let store = BlobStore::temp();
		store
			.insert(&SmolPath::from("scenes/foo/main.bsx"), "<ScenePrompt/>")
			.await
			.unwrap();
		store
			.insert(&SmolPath::from("scenes/foo/images/happy.png"), vec![1u8])
			.await
			.unwrap();
		store
			.insert(&SmolPath::from("scenes/foo/images/sad.png"), vec![2u8])
			.await
			.unwrap();
		store
			.insert(&SmolPath::from("scenes/bar/main.bsx"), "<ScenePrompt/>")
			.await
			.unwrap();
		let dir = SmolPath::from("scenes");
		// only dirs with a main.bsx, sorted.
		discover_scenes(&store, &dir)
			.await
			.unwrap()
			.xpect_eq(vec![SmolStr::new("bar"), SmolStr::new("foo")]);
		// images mapped to title (stem) + served url.
		let fallback = SmolPath::from("scenes/foo/images/happy.png");
		let images = list_scene_images(&store, &dir, "foo", &fallback).await.unwrap();
		images
			.iter()
			.map(|image| image.title.clone())
			.collect::<Vec<_>>()
			.xpect_eq(vec![SmolStr::new("happy"), SmolStr::new("sad")]);
		images[0].url.clone().xpect_eq(SmolStr::new("/scenes/foo/images/happy.png"));
		// an empty scene falls back to the configured image.
		let bar = list_scene_images(&store, &dir, "bar", &fallback).await.unwrap();
		bar.len().xpect_eq(1);
		bar[0].title.clone().xpect_eq(SmolStr::new("happy"));
	}
}

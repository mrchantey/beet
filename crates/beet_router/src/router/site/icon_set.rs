//! One source image, every size a browser or a PWA asks for, derived.
//!
//! `<IconSet src="assets/logo/logo.png"/>` at an entry's top level is the
//! whole declaration. Each output is a derived file whose provenance is the
//! source image's digest and nothing else, so a changed source regenerates the
//! set and an unchanged one costs one hash.
//!
//! # The set
//!
//! The modern minimum, and no more: a size nothing requests is a file to keep
//! current for no reader.
//!
//! | file | what asks for it |
//! | --- | --- |
//! | `favicon.ico` | holding 16, 32 and 48, for a browser that still asks for `/favicon.ico` by convention |
//! | `favicon.svg` | emitted only when the source is itself svg, so it scales rather than being traced |
//! | `icon-16.png`, `icon-32.png` | the tab and the bookmark bar |
//! | `icon-180.png` | `apple-touch-icon`, the iOS home screen |
//! | `icon-192.png`, `icon-512.png` | the web app manifest, the Android home screen and the splash |
//! | `icon-512-maskable.png` | the `maskable` purpose, below |
//! | `site.webmanifest` | lists the four png sizes with their purposes, and carries the name the entry's `PackageConfig` declares and, since a manifest names one colour, the light tint `<Head>` reads off the `Theme` resource for its light `theme-color` and its `msapplication-TileColor` (the light scheme `Primary` tone, `ThemeTints` beside `Theme`, where both read it) |
//!
//! # Maskable
//!
//! A maskable icon is cropped to whatever shape the platform wants, a circle or
//! a squircle, so anything outside the inscribed circle can be cut. The safe
//! zone is a circle of 80% of the width, centred, which means the source art is
//! scaled to 80% and padded to 512 on a background colour. The padding colour is
//! the theme's `Background` tone, so the icon reads as part of the brand rather
//! than floating on transparency that a platform renders black.
//!
//! The ordinary `icon-512.png` is NOT padded. Two files rather than one
//! compromise, because a padded icon in an unmasked slot looks small and a
//! full-bleed icon in a masked slot loses its corners.
//!
//! # The resize
//!
//! `image`, the one the workspace already carries, with `imageops::FilterType::Lanczos3`:
//! a 512 to 16 reduction is where a cheap filter turns a logo into mush, and the
//! derivation runs on a source change rather than per request, so the cost is
//! paid once.
//!
//! A source narrower than 512 in either dimension is an error naming the
//! smallest size it could serve, since an upscaled icon is worse than none and
//! a publication's icon must be at least 256 square.
//!
//! # The publication icon
//!
//! The 512, unpadded, which the standard.site publication record carries as a
//! blob (at least 256 square, under 1MB). The social card is a separate
//! declaration and stays one: it is wide, not square, and a crawler reads it as
//! meta rather than as a link.
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// `<IconSet src="assets/logo/logo.png"/>`: the module docs' set, derived
/// from the one source image and written beside it, in the store this entity
/// resolves by ancestry.
///
/// The declaration is its own producing action. Its [`Derivation`] is
/// `icon-set.json` beside the source, listing every file written
/// ([`DerivationOutputs`]), so `<Derive/>` regenerates a stale or incomplete
/// set and `beet check` fails a missing one. [`RouteHead`] links the set from
/// the declaration, each file at the url its store path serves at (`/` joined
/// to the path, as `<AssetsDir src="assets"/>` serves `assets/`).
///
/// Deriving needs the `site_images` feature; a binary without it, ie a served
/// site, still links the set it declares.
#[action]
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::component_hook(IconSet::declare_derivation))]
pub async fn IconSet(
	/// The source image, a raster at least 512 square or an svg.
	#[field]
	src: RelPath,
	cx: ActionContext<BlobStore>,
) -> Result<EntityBody> {
	let icon_set = IconSet { src };
	let bytes = cx.input.get(&icon_set.src).await?;
	let (theme, package) = cx
		.world()
		.with(|world: &mut World| {
			(
				world.get_resource::<Theme>().cloned().unwrap_or_default(),
				world
					.get_resource::<PackageConfig>()
					.cloned()
					.unwrap_or_default(),
			)
		})
		.await;
	cfg_if! {
		if #[cfg(feature = "site_images")] {
			icon_set.derive(&cx.input, &bytes, &theme, &package).await
		} else {
			let _ = (bytes, theme, package);
			bevybail!(
				"deriving the icon set from `{}` needs the `site_images` feature",
				icon_set.src
			)
		}
	}
}

/// One `<link rel="icon">` a document head carries for an [`IconSet`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconLink {
	/// `icon` or `apple-touch-icon`.
	pub rel: &'static str,
	/// The file's media type.
	pub mime: &'static str,
	/// The sizes the file holds, `any` for a vector.
	pub sizes: SmolStr,
	/// The url the file serves at.
	pub href: SmolStr,
}

impl IconSet {
	/// The derived document beside the source, listing the set.
	const DOCUMENT: &str = "icon-set.json";
	/// The multi-size favicon.
	const FAVICON_ICO: &str = "favicon.ico";
	/// The vector favicon, an svg source copied verbatim.
	const FAVICON_SVG: &str = "favicon.svg";
	/// The sizes `favicon.ico` holds.
	const ICO_SIZES: [u32; 3] = [16, 32, 48];
	/// The web app manifest.
	const MANIFEST: &str = "site.webmanifest";
	/// The largest size the set derives, and so the smallest source it accepts.
	const LARGEST: u32 = 512;

	/// The set derived from `src`.
	pub fn new(src: impl Into<RelPath>) -> Self { Self { src: src.into() } }

	/// The [`Derivation`] this set declares: its document beside the source.
	pub fn derivation(&self) -> Derivation {
		Derivation::new(self.path(Self::DOCUMENT)).with_input(self.src.clone())
	}

	/// The store path of the 512, unpadded, which a standard.site publication
	/// carries as its icon.
	pub fn publication_icon(&self) -> RelPath { self.png(Self::LARGEST) }

	/// Whether the source is svg, which adds `favicon.svg` to the set.
	pub fn is_svg(&self) -> bool {
		self.src
			.as_smol_path()
			.extension()
			.is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
	}

	/// The icon links a document head carries for the set, the vector first
	/// so a browser that reads it prefers it.
	pub fn links(&self) -> Vec<IconLink> {
		let link = |rel, mime, sizes: String, path: RelPath| IconLink {
			rel,
			mime,
			sizes: sizes.into(),
			href: Self::href(&path),
		};
		let mut links = Vec::new();
		if self.is_svg() {
			links.push(link(
				"icon",
				"image/svg+xml",
				"any".into(),
				self.path(Self::FAVICON_SVG),
			));
		}
		links.push(link(
			"icon",
			"image/x-icon",
			Self::ICO_SIZES
				.iter()
				.map(|size| format!("{size}x{size}"))
				.collect::<Vec<_>>()
				.join(" "),
			self.path(Self::FAVICON_ICO),
		));
		for size in [16, 32] {
			links.push(link(
				"icon",
				"image/png",
				format!("{size}x{size}"),
				self.png(size),
			));
		}
		links.push(link(
			"apple-touch-icon",
			"image/png",
			"180x180".into(),
			self.png(180),
		));
		links
	}

	/// The url the web app manifest serves at.
	pub fn manifest_href(&self) -> SmolStr {
		Self::href(&self.path(Self::MANIFEST))
	}

	/// The url a file at `path` in the store serves at.
	pub(crate) fn href(path: &RelPath) -> SmolStr { format!("/{path}").into() }

	/// The file `name` beside the source.
	fn path(&self, name: &str) -> RelPath {
		match self.src.parent() {
			Some(dir) => dir.join(name),
			None => RelPath::new(name),
		}
	}

	/// The unpadded png of `size`.
	fn png(&self, size: u32) -> RelPath {
		self.path(&format!("icon-{size}.png"))
	}

	/// The hook landing this set's [`Derivation`] on its entity.
	fn declare_derivation(&self) -> impl FnOnce(&mut EntityCommands) + use<> {
		let derivation = self.derivation();
		move |entity: &mut EntityCommands| {
			entity.insert(derivation);
		}
	}
}

#[cfg(feature = "site_images")]
impl IconSet {
	/// The unpadded png sizes, each `icon-<size>.png`.
	const PNG_SIZES: [u32; 5] = [16, 32, 180, 192, 512];
	/// The padded 512 for the `maskable` purpose.
	const MASKABLE: &str = "icon-512-maskable.png";
	/// The maskable safe zone, as a fraction of the width.
	const SAFE_ZONE: f32 = 0.8;

	/// Write every file of the set into `store` from the source `bytes`,
	/// answering the document: the source's provenance and the files written.
	async fn derive(
		&self,
		store: &BlobStore,
		bytes: &[u8],
		theme: &Theme,
		package: &PackageConfig,
	) -> Result<EntityBody> {
		let source = IconSource::decode(self, bytes)?;
		let background = theme
			.resolve(ColorScheme::Light, material::colors::Background)?
			.to_srgba();
		let tints = ThemeTints::new(theme)?;
		let mut outputs = DerivationOutputs::default();
		for size in Self::PNG_SIZES {
			outputs
				.write(store, self.png(size), source.png(size)?)
				.await?;
		}
		outputs
			.write(
				store,
				self.path(Self::MASKABLE),
				source.maskable(background.to_u8_array())?,
			)
			.await?;
		outputs
			.write(store, self.path(Self::FAVICON_ICO), source.ico()?)
			.await?;
		if self.is_svg() {
			outputs
				.write(store, self.path(Self::FAVICON_SVG), bytes.to_vec())
				.await?;
		}
		outputs
			.write(
				store,
				self.path(Self::MANIFEST),
				self.manifest(package, &tints, &background.to_hex())?,
			)
			.await?;
		EntityBody::default()
			.with_typed(&Provenance::new([ProvenanceSource::file(
				self.src.clone(),
				bytes,
			)]))?
			.with_typed(&outputs)
	}

	/// The web app manifest, its icons relative to it since it sits beside
	/// them.
	fn manifest(
		&self,
		package: &PackageConfig,
		tints: &ThemeTints,
		background: &str,
	) -> Result<Vec<u8>> {
		let icon = |name: String, size: u32, purpose: &str| {
			serde_json::json!({
				"src": name,
				"sizes": format!("{size}x{size}"),
				"type": "image/png",
				"purpose": purpose,
			})
		};
		let mut icons = [180, 192, 512]
			.into_iter()
			.map(|size| icon(format!("icon-{size}.png"), size, "any"))
			.collect::<Vec<_>>();
		icons.push(icon(Self::MASKABLE.into(), Self::LARGEST, "maskable"));
		serde_json::to_vec_pretty(&serde_json::json!({
			"name": package.title,
			"short_name": package.title,
			"description": package.description,
			"start_url": "/",
			"display": "standalone",
			"theme_color": tints.light.as_str(),
			"background_color": background,
			"icons": icons,
		}))?
		.xok()
	}
}

/// A decoded icon source, rendered at each size of the set, and at the size
/// a social card places the logo.
#[cfg(feature = "site_images")]
pub(crate) enum IconSource {
	/// A raster, resized down.
	Raster(image::RgbaImage),
	/// A vector, rasterised at each size.
	Svg(resvg::usvg::Tree),
}

#[cfg(feature = "site_images")]
impl IconSource {
	/// Decode `bytes` as `icon_set`'s source, refusing a raster too small for
	/// the set.
	pub(crate) fn decode(icon_set: &IconSet, bytes: &[u8]) -> Result<Self> {
		if icon_set.is_svg() {
			let options = resvg::usvg::Options::default();
			return Self::Svg(resvg::usvg::Tree::from_data(bytes, &options)?)
				.xok();
		}
		let image = image::load_from_memory(bytes)?.into_rgba8();
		let (width, height) = image.dimensions();
		if width.min(height) < IconSet::LARGEST {
			let serves = IconSet::PNG_SIZES
				.iter()
				.rev()
				.find(|size| **size <= width.min(height))
				.map(|size| format!("icon-{size}.png at most"))
				.unwrap_or_else(|| "no size of the set".into());
			let (src, largest) = (&icon_set.src, IconSet::LARGEST);
			bevybail!(
				"icon source `{src}` is {width}x{height}, under the \
				 {largest}x{largest} the set needs: it could serve {serves}"
			);
		}
		Self::Raster(image).xok()
	}

	/// The source fit centred in a `size` square, transparent around a source
	/// that is not square.
	pub(crate) fn render(&self, size: u32) -> Result<image::RgbaImage> {
		match self {
			Self::Raster(image) => {
				let (width, height) = image.dimensions();
				let scale = size as f32 / width.max(height) as f32;
				let fit =
					|side: u32| ((side as f32 * scale).round() as u32).max(1);
				let resized = image::imageops::resize(
					image,
					fit(width),
					fit(height),
					image::imageops::FilterType::Lanczos3,
				);
				let mut canvas = image::RgbaImage::new(size, size);
				image::imageops::overlay(
					&mut canvas,
					&resized,
					((size - resized.width()) / 2).into(),
					((size - resized.height()) / 2).into(),
				);
				canvas.xok()
			}
			Self::Svg(tree) => {
				use resvg::tiny_skia;
				let tree_size = tree.size();
				let scale =
					size as f32 / tree_size.width().max(tree_size.height());
				let mut pixmap = tiny_skia::Pixmap::new(size, size)
					.ok_or_else(|| bevyhow!("no {size}x{size} pixmap"))?;
				resvg::render(
					tree,
					tiny_skia::Transform::from_scale(scale, scale)
						.post_translate(
							(size as f32 - tree_size.width() * scale) / 2.,
							(size as f32 - tree_size.height() * scale) / 2.,
						),
					&mut pixmap.as_mut(),
				);
				// tiny-skia holds premultiplied alpha, an image straight alpha
				let pixels = pixmap
					.pixels()
					.iter()
					.flat_map(|pixel| {
						let color = pixel.demultiply();
						[
							color.red(),
							color.green(),
							color.blue(),
							color.alpha(),
						]
					})
					.collect();
				image::RgbaImage::from_raw(size, size, pixels)
					.ok_or_else(|| bevyhow!("a {size}x{size} raster"))
			}
		}
	}

	/// The source at `size`, as png.
	fn png(&self, size: u32) -> Result<Vec<u8>> {
		Self::encode_png(&self.render(size)?)
	}

	/// The 512 with the source scaled into the safe zone, padded on
	/// `background`, as png.
	fn maskable(&self, background: [u8; 4]) -> Result<Vec<u8>> {
		let size = IconSet::LARGEST;
		let art =
			self.render((size as f32 * IconSet::SAFE_ZONE).round() as u32)?;
		let mut canvas =
			image::RgbaImage::from_pixel(size, size, image::Rgba(background));
		let offset = ((size - art.width()) / 2).into();
		image::imageops::overlay(&mut canvas, &art, offset, offset);
		Self::encode_png(&canvas)
	}

	/// `favicon.ico`, each of [`IconSet::ICO_SIZES`] a png frame.
	fn ico(&self) -> Result<Vec<u8>> {
		let frames = IconSet::ICO_SIZES
			.iter()
			.map(|size| -> Result<_> {
				image::codecs::ico::IcoFrame::as_png(
					self.render(*size)?.as_raw(),
					*size,
					*size,
					image::ExtendedColorType::Rgba8,
				)?
				.xok()
			})
			.collect::<Result<Vec<_>>>()?;
		let mut bytes = Vec::new();
		image::codecs::ico::IcoEncoder::new(&mut bytes)
			.encode_images(&frames)?;
		bytes.xok()
	}

	/// `image` as png.
	pub(crate) fn encode_png(image: &image::RgbaImage) -> Result<Vec<u8>> {
		let mut bytes = std::io::Cursor::new(Vec::new());
		image.write_to(&mut bytes, image::ImageFormat::Png)?;
		bytes.into_inner().xok()
	}
}

#[cfg(all(test, feature = "site_images"))]
mod test {
	use super::IconSource;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	const SRC: &str = "branding/logo.png";

	/// A `width` by `height` png, a diagonal gradient so a resize has detail
	/// to lose, its tint `seed`.
	fn png(width: u32, height: u32, seed: u8) -> Vec<u8> {
		let image = image::RgbaImage::from_fn(width, height, |x, y| {
			image::Rgba([(x % 256) as u8, (y % 256) as u8, seed, 255])
		});
		IconSource::encode_png(&image).unwrap()
	}

	/// A world declaring the set from [`SRC`] in a temp store holding `source`.
	fn setup(source: Vec<u8>) -> (World, Entity, BlobStore) {
		let mut world = AsyncPlugin.into_world();
		let store = BlobStore::temp();
		let entity = world.spawn(IconSet::new(SRC)).flush();
		let inserted = store.clone();
		async_ext::block_on(async move {
			inserted.insert(&RelPath::new(SRC), source).await
		})
		.unwrap();
		(world, entity, store)
	}

	/// [`Derivation::ensure`] in `world`'s async context.
	async fn ensure(
		world: &mut World,
		entity: Entity,
		store: &BlobStore,
	) -> Freshness {
		let store = store.clone();
		world
			.run_async_then(async move |world| {
				Derivation::ensure(&world, entity, &store).await
			})
			.await
			.unwrap()
	}

	/// The outputs the set's document lists.
	async fn outputs(store: &BlobStore) -> DerivationOutputs {
		IconSet::new(SRC)
			.derivation()
			.read(store)
			.await
			.unwrap()
			.map(|body| DerivationOutputs::of(&body).unwrap())
			.unwrap()
	}

	/// The image at `path` in `store`.
	async fn image(store: &BlobStore, path: &str) -> image::RgbaImage {
		image::load_from_memory(&store.get(&RelPath::new(path)).await.unwrap())
			.unwrap()
			.into_rgba8()
	}

	#[beet_core::test]
	async fn derives_the_set_idempotently() {
		// wider than tall, so the square icons letterbox it
		let (mut world, entity, store) = setup(png(768, 512, 0));
		ensure(&mut world, entity, &store)
			.await
			.xpect_eq(Freshness::Missing);
		let first = outputs(&store).await;
		first
			.0
			.iter()
			.map(|output| output.path.to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"branding/icon-16.png",
				"branding/icon-32.png",
				"branding/icon-180.png",
				"branding/icon-192.png",
				"branding/icon-512.png",
				"branding/icon-512-maskable.png",
				"branding/favicon.ico",
				"branding/site.webmanifest",
			]);
		ensure(&mut world, entity, &store)
			.await
			.xpect_eq(Freshness::Fresh);
		// a regeneration from the same source writes the same bytes
		store
			.remove(&IconSet::new(SRC).derivation().path)
			.await
			.unwrap();
		ensure(&mut world, entity, &store).await;
		outputs(&store).await.xpect_eq(first);
	}

	#[beet_core::test]
	async fn regenerates_a_changed_source() {
		let (mut world, entity, store) = setup(png(512, 512, 0));
		ensure(&mut world, entity, &store).await;
		let first = outputs(&store).await;
		store
			.insert(&RelPath::new(SRC), png(512, 512, 200))
			.await
			.unwrap();
		ensure(&mut world, entity, &store)
			.await
			.to_string()
			.xpect_eq(format!("file `{SRC}` changed"));
		(outputs(&store).await != first).xpect_true();
	}

	#[beet_core::test]
	async fn restores_a_deleted_icon() {
		let (mut world, entity, store) = setup(png(512, 512, 0));
		ensure(&mut world, entity, &store).await;
		let icon = RelPath::new("branding/icon-16.png");
		store.remove(&icon).await.unwrap();
		ensure(&mut world, entity, &store)
			.await
			.xpect_eq(Freshness::OutputMissing(icon.clone()));
		image(&store, "branding/icon-16.png")
			.await
			.dimensions()
			.xpect_eq((16, 16));
	}

	/// The maskable icon pads the art onto the theme's background, the
	/// ordinary one does not.
	#[beet_core::test]
	async fn pads_the_maskable_icon() {
		let (mut world, entity, store) = setup(png(512, 512, 0));
		ensure(&mut world, entity, &store).await;
		let background = Theme::default()
			.resolve(ColorScheme::Light, material::colors::Background)
			.unwrap()
			.to_srgba()
			.to_u8_array();
		image(&store, "branding/icon-512-maskable.png")
			.await
			.get_pixel(0, 0)
			.0
			.xpect_eq(background);
		image(&store, "branding/icon-512.png")
			.await
			.get_pixel(0, 0)
			.0
			.xpect_eq([0, 0, 0, 255]);
		let manifest = store
			.get(&RelPath::new("branding/site.webmanifest"))
			.await
			.unwrap();
		str::from_utf8(&manifest)
			.unwrap()
			.xpect_contains("\"purpose\": \"maskable\"")
			.xpect_contains(&PackageConfig::default().title.to_string());
	}

	#[beet_core::test]
	async fn refuses_a_small_source() {
		let (mut world, entity, store) = setup(png(200, 300, 0));
		let store = store.clone();
		world
			.run_async_then(async move |world| {
				Derivation::ensure(&world, entity, &store).await
			})
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("200x300")
			.xpect_contains("icon-192.png at most");
	}

	/// Every file the head links is one the set wrote.
	#[beet_core::test]
	async fn head_links_match_the_set() {
		let (mut world, entity, store) = setup(png(512, 512, 0));
		ensure(&mut world, entity, &store).await;
		let outputs = outputs(&store).await;
		let icon_set = IconSet::new(SRC);
		for href in icon_set
			.links()
			.into_iter()
			.map(|link| link.href)
			.chain([icon_set.manifest_href()])
		{
			outputs
				.get(&RelPath::new(href.trim_start_matches('/')))
				.is_some()
				.xpect_true();
		}
	}

	/// An svg source is rasterised at each size and served as itself too.
	#[beet_core::test]
	async fn derives_an_svg_source() {
		const SVG: &str = "branding/logo.svg";
		let mut world = AsyncPlugin.into_world();
		let store = BlobStore::temp();
		store
			.insert(
				&RelPath::new(SVG),
				r##"<svg width="150" height="100" xmlns="http://www.w3.org/2000/svg"><rect width="150" height="100" fill="#f00"/></svg>"##,
			)
			.await
			.unwrap();
		let entity = world.spawn(IconSet::new(SVG)).flush();
		ensure(&mut world, entity, &store).await;
		let icon = image(&store, "branding/icon-512.png").await;
		// letterboxed: transparent above the art, red within it
		icon.get_pixel(256, 10).0[3].xpect_eq(0);
		icon.get_pixel(256, 256).0.xpect_eq([255, 0, 0, 255]);
		IconSet::new(SVG).links()[0]
			.href
			.xpect_eq("/branding/favicon.svg");
		store
			.exists(&RelPath::new("branding/favicon.svg"))
			.await
			.unwrap()
			.xpect_true();
	}
}

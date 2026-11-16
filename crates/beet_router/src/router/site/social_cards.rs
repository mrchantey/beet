//! A link preview for every page, drawn from what the page already declares.
//!
//! `<SocialCards font="assets/fonts/InterVariable.ttf"/>` inside the served
//! `<Router>`, beside `<Sitemap/>`, is the whole declaration. Every page the
//! sitemap lists gets a 1200x630 card, written to `assets/og/<route>.png` (the
//! root route's `index.png`) beside the one document listing the set,
//! `assets/og/social-cards.json`. The cards are derived binaries like the icon
//! set, stored in the bucket-backed `site/assets`, so a preview costs nothing
//! per request and the served binary needs no renderer.
//!
//! # The card
//!
//! One frame on every card, so every preview of a site is recognisable in a
//! feed before it is read: the light `Background` tone, a `Primary` band along
//! the foot, the title in the font's bold (at most three lines, ellipsized),
//! the description in the `OnSurfaceVariant` tone (at most two lines), and
//! along the bottom the logo, the site title and the homepage's host. The
//! logo is the source image of the world's one [`IconSet`], so a site
//! declares it once.
//!
//! A page with no media spans its text across the card. A page with media
//! narrows the text to the left column and fills one fixed slot on the right
//! with it, centre-cropped to the slot's aspect and never letterboxed, which
//! also crops away the bars a 4:3 YouTube poster bakes around a 16:9 frame. A
//! video's poster wears a play badge. The cost, accepted: a platform printing
//! the title under the image shows it twice.
//!
//! The text wraps by the font's own glyph advances, measured with `skrifa`,
//! the reader `resvg`'s text layout shapes with, at the weight and optical
//! size the card sets it in; never by character count, which a proportional
//! face makes a guess. The layout is code, and the font is the one file the
//! declaration names, so the card renders the same on every machine.
//!
//! # The media
//!
//! [`DocMeta::social_image_url`](beet_ui::prelude::DocMeta::social_image_url):
//! the page's own image, else its video's poster. A site-rooted image
//! (`/assets/..`) is read from the store the cards are derived in, at the path
//! `<AssetsDir>` serves it from, a page-relative one resolved against the
//! page's url first, as a browser resolves it. A remote one is fetched once
//! per derivation. An image that cannot be read or fetched fails the set
//! naming the page and the url, never a card silently without its media.
//!
//! # What regenerates the set
//!
//! The provenance names the font, the logo, each page's source file and each
//! embedded image, a file by its digest and a remote one by the digest of what
//! it answered ([`ProvenanceSource::Url`], counted unchanged since nothing
//! local can hash it without fetching it again). Beside the files, the
//! declaration computes a value per card from what the card draws off the
//! world (the page's title, description and media url) and one for the frame
//! (the site title, description, homepage and `Theme`), so a page declared in
//! code or a rebrand regenerates the set like an edited file
//! ([`ProvenanceValue`]). A page added is a value the document was not
//! computed from and a page removed one it no longer declares, so both
//! regenerate too, and the removed page's card is pruned as a superseded
//! output. One document rather than one per page: a regeneration redraws
//! every card, which costs little at a site's size.
//!
//! # The head
//!
//! [`RouteHead`] hands `<Head>` the card's absolute url as the page's social
//! image, ahead of the page's own image, which a site without `<SocialCards/>`
//! still previews with. A page outside the set (unlisted, say) previews with
//! the root route's card, so every page of a site with cards previews as a
//! card. The standard.site `coverImage` stays the page's raw image, since a
//! reader renders it above the title and a card would repeat it.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// `<SocialCards font="assets/fonts/InterVariable.ttf"/>`: the module docs'
/// card for every page its [`Router`] lists, derived into `assets/og` in the
/// store this entity resolves by ancestry.
///
/// The declaration is its own producing action. Its [`Derivation`] is
/// `assets/og/social-cards.json`, listing every card written
/// ([`DerivationOutputs`]), so `<Derive/>` regenerates a stale or incomplete
/// set and `beet check` fails a missing one. It covers its router's whole url
/// space wherever it sits, and a world declares at most one.
///
/// Deriving needs the `site_images` feature; a binary without it, ie a served
/// site, still points each page's head at its card.
#[action]
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::component_hook(SocialCards::declare_derivation))]
pub async fn SocialCards(
	/// The font the text is set in, a TTF or OTF in the store: `fontdb` reads
	/// neither `woff` nor `woff2`. A variable face serves both weights.
	#[field]
	font: RelPath,
	cx: ActionContext<BlobStore>,
) -> Result<EntityBody> {
	let cards = SocialCards { font };
	cfg_if! {
		if #[cfg(feature = "site_images")] {
			let entity = cx.id();
			let set = cx
				.world()
				.with(move |world: &mut World| CardSet::collect(world, entity))
				.await?;
			cards.derive(&cx.input, set).await
		} else {
			let _ = (cards, cx);
			bevybail!(
				"deriving the social cards needs the `site_images` feature"
			)
		}
	}
}

impl SocialCards {
	/// The directory the set is written to, in the store.
	const DIR: &str = "assets/og";
	/// The derived document listing the set.
	const DOCUMENT: &str = "social-cards.json";

	/// The cards set in `font`.
	pub fn new(font: impl Into<RelPath>) -> Self { Self { font: font.into() } }

	/// The [`Derivation`] this set declares, its document in `assets/og`
	/// naming the font; the logo, the pages and the values join it as the
	/// world names them ([`refresh_inputs`](Self::refresh_inputs)).
	pub fn derivation(&self) -> Derivation {
		Derivation::new(RelPath::new(Self::DIR).join(Self::DOCUMENT))
			.with_input(self.font.clone())
	}

	/// The store path of the card for the page at route `path`.
	pub fn card(path: &RelPath) -> RelPath {
		let name = match path.as_str() {
			"" => "index",
			path => path,
		};
		RelPath::new(Self::DIR).join(format!("{name}.png"))
	}

	/// The url of the card a page at route `route` previews with: its own when
	/// `listed` holds it, else the root route's, else none.
	fn card_href(listed: &[SyndicationPage], route: Entity) -> Option<SmolStr> {
		listed
			.iter()
			.find(|page| page.route == route)
			.or_else(|| {
				listed.iter().find(|page| page.path.as_str().is_empty())
			})
			.map(|page| IconSet::href(&Self::card(&page.path)))
	}

	/// The hook landing this set's [`Derivation`] on its entity, then the
	/// sources it reads once the world can name them.
	fn declare_derivation(&self) -> impl FnOnce(&mut EntityCommands) + use<> {
		let derivation = self.derivation();
		move |entity: &mut EntityCommands| {
			entity.insert(derivation);
			entity.commands().queue(Self::refresh_inputs);
		}
	}

	/// Name every source each declared set reads as its [`Derivation`]'s
	/// inputs (the font, the logo, each listed page's source file) and every
	/// card's drawn text as its values, so a page added, edited in code or
	/// rebranded since the set was derived is stale. A set whose router has
	/// no tree yet keeps the font and the frame alone until one lands.
	fn refresh_inputs(world: &mut World) -> Result {
		let derivations =
			world
				.with_state::<(Query<(Entity, &SocialCards)>, SocialCardsQuery), _>(
					|(cards, query)| {
						cards
							.iter()
							.map(|(entity, cards)| {
								let derivation =
									query.inputs(entity).into_iter().fold(
										cards.derivation(),
										Derivation::with_input,
									);
								let derivation = query
									.values(entity)
									.into_iter()
									.fold(derivation, Derivation::with_value);
								(entity, derivation)
							})
							.collect::<Vec<_>>()
					},
				);
		for (entity, derivation) in derivations {
			if world.get::<Derivation>(entity) != Some(&derivation) {
				world.entity_mut(entity).insert(derivation);
			}
		}
		Ok(())
	}
}

impl TypeInstaller for SocialCards {
	/// Registers the declaration and refreshes each set's inputs whenever
	/// something a card draws lands: a route tree, a page's metadata, the
	/// logo, the theme or the package.
	fn install(app: &mut App) {
		app.register_type::<Self>()
			.add_observer(queue_refresh::<RouteTree>)
			.add_observer(queue_refresh::<PageMeta>)
			.add_observer(queue_refresh::<IconSet>)
			.add_observer(queue_refresh::<Theme>)
			.add_observer(queue_refresh::<PackageConfig>);
	}
}

/// Observer queueing [`SocialCards::refresh_inputs`] once `T` lands, a
/// resource included, so the refresh reads the world whole.
fn queue_refresh<T: Component>(_ev: On<Insert<T>>, mut commands: Commands) {
	commands.queue(SocialCards::refresh_inputs);
}

/// The url of the card the page at `route` previews with, when the world
/// declares a [`SocialCards`] set: see [`RouteHead`].
#[derive(SystemParam)]
pub(crate) struct SocialCardQuery<'w, 's> {
	cards: Query<'w, 's, Entity, With<SocialCards>>,
	syndication: SyndicationQuery<'w, 's>,
}

impl SocialCardQuery<'_, '_> {
	/// The absolute url of `route`'s card, `None` without a set, outside its
	/// url space or with no `homepage` to resolve against, since a crawler
	/// needs an absolute `og:image`.
	///
	/// # Errors
	/// Errors when the world declares more than one set.
	pub fn card_url(&self, route: Entity) -> Result<Option<Url>> {
		let cards = match self.cards.iter().collect::<Vec<_>>().as_slice() {
			[] => return Ok(None),
			[cards] => *cards,
			_ => bevybail!(
				"more than one `SocialCards` declared: a site has one set of cards"
			),
		};
		let Ok(listed) = self.syndication.listed(cards, &default()) else {
			return Ok(None);
		};
		SocialCards::card_href(&listed, route)
			.and_then(|href| {
				self.syndication.package().absolute_url(&href).ok()
			})
			.xok()
	}
}

/// What a [`SocialCards`] set reads off the world: its pages, each with the
/// file it was discovered from, the logo and the theme.
#[derive(SystemParam)]
struct SocialCardsQuery<'w, 's> {
	syndication: SyndicationQuery<'w, 's>,
	theme: Option<Res<'w, Theme>>,
	blobs: Query<'w, 's, &'static Blob>,
	stores: AncestorQuery<'w, 's, &'static BlobStore>,
	icon_sets: Query<'w, 's, &'static IconSet>,
}

impl SocialCardsQuery<'_, '_> {
	/// Every page the set at `cards` draws a card for, in route-tree order,
	/// each with its source file.
	fn pages(
		&self,
		cards: Entity,
	) -> Result<Vec<(SyndicationPage, Option<RelPath>)>> {
		let store = self.stores.get(cards)?;
		self.syndication
			.listed(cards, &default())?
			.into_iter()
			.map(|page| -> Result<_> {
				let source = self.source(store, &page)?;
				(page, source).xok()
			})
			.collect()
	}

	/// The sources the set at `cards` reads beside its font, as far as the
	/// world can name them now: the logo and each page's source file.
	fn inputs(&self, cards: Entity) -> Vec<RelPath> {
		let logo = self.logo().ok().flatten().map(|logo| logo.src);
		let sources = self
			.pages(cards)
			.into_iter()
			.flatten()
			.filter_map(|(_, source)| source);
		logo.into_iter().chain(sources).collect()
	}

	/// The values the set at `cards` draws off the world, as far as it can
	/// name them now: the frame, then each card's text and media url, named
	/// by the card's path. A value is the drawn fields joined, so it changes
	/// exactly when the drawing could.
	fn values(&self, cards: Entity) -> Vec<ProvenanceValue> {
		let package = self.syndication.package();
		let homepage = package.homepage.as_ref().map(Url::to_string);
		let theme =
			format!("{:?}", self.theme.as_deref().cloned().unwrap_or_default());
		let frame = Self::value("frame", [
			Some(package.title.as_str()),
			Some(package.description.as_str()),
			homepage.as_deref(),
			Some(theme.as_str()),
		]);
		let pages = self.pages(cards).into_iter().flatten().map(|(page, _)| {
			let card = SocialCards::card(&page.path);
			// a poster url is only ever a video's, so the url implies the badge
			let media = page.meta.social_image_url().map(|url| url.to_string());
			Self::value(card.as_str(), [
				page.meta.title.as_deref(),
				page.meta.description.as_deref(),
				media.as_deref(),
			])
		});
		core::iter::once(frame).chain(pages).collect()
	}

	/// The value `name` over `fields`, each delimited so no two sets of
	/// fields join alike.
	fn value<'a>(
		name: &str,
		fields: impl IntoIterator<Item = Option<&'a str>>,
	) -> ProvenanceValue {
		let bytes = fields
			.into_iter()
			.map(|field| match field {
				Some(field) => format!("+{}:{field}", field.len()),
				None => "-".to_string(),
			})
			.collect::<String>();
		ProvenanceValue::new(name, bytes.as_bytes())
	}

	/// The world's one [`IconSet`], the logo every card wears.
	fn logo(&self) -> Result<Option<IconSet>> {
		match self.icon_sets.iter().collect::<Vec<_>>().as_slice() {
			[] => None,
			[icon_set] => Some((*icon_set).clone()),
			_ => bevybail!(
				"more than one `IconSet` declared: a site has one set of icons"
			),
		}
		.xok()
	}

	/// `page`'s source file relative to `store`, the one the cards are
	/// derived in: its [`Blob`]'s path joined onto the subdirectory its store
	/// is scoped to beneath `store`.
	///
	/// # Errors
	/// Errors when the source lives in a store `store` does not hold, since
	/// the set could not see it change.
	fn source(
		&self,
		store: &BlobStore,
		page: &SyndicationPage,
	) -> Result<Option<RelPath>> {
		let Ok(blob) = self.blobs.get(page.route) else {
			return Ok(None);
		};
		let subdir = (blob.store().root_key() == store.root_key())
			.then(|| blob.store().subdir().strip_prefix(&store.subdir()))
			.flatten()
			.ok_or_else(|| {
				bevyhow!(
					"page `{}` is read from `{}`, outside the store the social \
					 cards are derived in, so its card could not track it",
					page.path,
					blob.path()
				)
			})?;
		subdir.join(blob.path().as_str()).xmap(Some).xok()
	}
}

/// Everything a set is drawn from, cloned out of the world so the drawing
/// holds no world access.
#[cfg(feature = "site_images")]
struct CardSet {
	/// Every page the set draws a card for, in route-tree order.
	pages: Vec<CardPage>,
	/// The logo, the world's [`IconSet`].
	logo: Option<IconSet>,
	/// The palette the frame is drawn in.
	theme: Theme,
	/// The site title, the homepage and the fallback description.
	package: PackageConfig,
}

/// One page a card is drawn for.
#[cfg(feature = "site_images")]
struct CardPage {
	/// The route path, ie `blog/ecs-router`.
	path: RelPath,
	/// The page's authored metadata.
	meta: DocMeta,
	/// The page's source file in the store the cards are derived in, `None`
	/// for a page declared in code.
	source: Option<RelPath>,
}

#[cfg(feature = "site_images")]
impl CardSet {
	/// Everything the set at `cards` is drawn from.
	fn collect(world: &mut World, cards: Entity) -> Result<Self> {
		let (pages, logo) = world.with_state::<SocialCardsQuery, _>(|query| {
			(query.pages(cards), query.logo())
		});
		Self {
			pages: pages?
				.into_iter()
				.map(|(page, source)| CardPage {
					path: page.path,
					meta: page.meta,
					source,
				})
				.collect(),
			logo: logo?,
			theme: world.get_resource::<Theme>().cloned().unwrap_or_default(),
			package: world
				.get_resource::<PackageConfig>()
				.cloned()
				.unwrap_or_default(),
		}
		.xok()
	}
}

#[cfg(feature = "site_images")]
impl SocialCards {
	/// Draw every card of `set` into `store`, answering the document: what each
	/// was drawn from and the cards written.
	async fn derive(
		&self,
		store: &BlobStore,
		set: CardSet,
	) -> Result<EntityBody> {
		let mut sources = CardSources::default();
		let font = CardFont::new(sources.file(store, &self.font).await?)?;
		let logo = match &set.logo {
			Some(icon_set) => sources
				.file(store, &icon_set.src)
				.await?
				.xmap(|bytes| {
					super::icon_set::IconSource::decode(icon_set, &bytes)
				})?
				.render(CardLayout::LOGO as u32)?
				.xmap(|logo| super::icon_set::IconSource::encode_png(&logo))?
				.xmap(Some),
			None => None,
		};
		let palette = CardPalette::new(&set.theme)?;
		let mut outputs = DerivationOutputs::default();
		for page in &set.pages {
			if let Some(source) = &page.source {
				sources.file(store, source).await?;
			}
			let media = match page.meta.social_image_url() {
				Some(url) => sources
					.media(store, page, url)
					.await?
					.xmap(|bytes| CardMedia::new(page, &bytes))?
					.xmap(Some),
				None => None,
			};
			let card = Card::new(&font, &set.package, page, media)?;
			outputs
				.write(
					store,
					Self::card(&page.path),
					card.render(&font, &palette, logo.as_deref())?,
				)
				.await?;
		}
		EntityBody::default()
			.with(&Provenance::new(sources.0))?
			.with(&outputs)
	}
}

/// The sources a set was drawn from, each listed once.
#[cfg(feature = "site_images")]
#[derive(Default)]
struct CardSources(Vec<ProvenanceSource>);

#[cfg(feature = "site_images")]
impl CardSources {
	/// The file at `path` in `store`, listed as a source.
	async fn file(
		&mut self,
		store: &BlobStore,
		path: &RelPath,
	) -> Result<Vec<u8>> {
		let bytes = store.get(path).await?.to_vec();
		self.push(ProvenanceSource::file(path.clone(), &bytes));
		bytes.xok()
	}

	/// The image `url` names for `page`: a site-rooted one read from `store`
	/// at the path it serves from, a remote one fetched, either listed as a
	/// source. A remote one already listed is fetched again, since a set
	/// rarely shares one between pages.
	async fn media(
		&mut self,
		store: &BlobStore,
		page: &CardPage,
		url: Url,
	) -> Result<Vec<u8>> {
		if !url.is_external() {
			let path = match url.is_rooted() {
				true => RelPath::from_segments(url.path()),
				// a page-relative image resolves beside the page, as a
				// browser resolves it against the page's url
				false => page
					.path
					.parent()
					.unwrap_or_default()
					.join(url.path().join("/")),
			};
			return self.file(store, &path).await.map_err(|err| {
				bevyhow!(
					"page `{}`'s image `{url}` is not in the store: {err}",
					page.path
				)
			});
		}
		let fetched = async {
			Request::get(url.to_string())
				.send()
				.await?
				.into_result()
				.await?
				.bytes()
				.await?
				.to_vec()
				.xok::<BevyError>()
		}
		.await
		.map_err(|err| {
			bevyhow!(
				"page `{}`'s image `{url}` could not be fetched: {err}",
				page.path
			)
		})?;
		self.push(ProvenanceSource::url(url, &fetched));
		fetched.xok()
	}

	/// List `source` unless it already is.
	fn push(&mut self, source: ProvenanceSource) {
		if !self.0.contains(&source) {
			self.0.push(source);
		}
	}
}

/// The font a card is set in: its bytes for measuring and the family name an
/// svg names it by.
#[cfg(feature = "site_images")]
struct CardFont {
	/// The face database the render resolves the family from.
	fontdb: std::sync::Arc<resvg::usvg::fontdb::Database>,
	/// The font file.
	bytes: Vec<u8>,
	/// The family the font file declares, ie `Inter Variable`.
	family: String,
}

#[cfg(feature = "site_images")]
impl CardFont {
	/// The font in `bytes`, refused when it holds no face `fontdb` reads.
	fn new(bytes: Vec<u8>) -> Result<Self> {
		let mut fontdb = resvg::usvg::fontdb::Database::new();
		fontdb.load_font_data(bytes.clone());
		let family = fontdb
			.faces()
			.next()
			.and_then(|face| face.families.first())
			.map(|(family, _)| family.clone())
			.ok_or_else(|| {
				bevyhow!(
					"the social card font holds no face `fontdb` reads: a card \
					 needs a TTF or OTF, never a woff"
				)
			})?;
		Self {
			fontdb: fontdb.into(),
			bytes,
			family,
		}
		.xok()
	}

	/// The advance width of `text` set at `size` and `weight`, at the optical
	/// size `usvg` picks for that size, as it renders.
	fn width(&self, text: &str, size: f32, weight: f32) -> Result<f32> {
		use skrifa::MetadataProvider;
		let font = skrifa::FontRef::new(&self.bytes)?;
		let location = font.axes().location([("wght", weight), ("opsz", size)]);
		let metrics =
			font.glyph_metrics(skrifa::instance::Size::new(size), &location);
		let charmap = font.charmap();
		text.chars()
			.filter_map(|ch| charmap.map(ch))
			.filter_map(|glyph| metrics.advance_width(glyph))
			.sum::<f32>()
			.xok()
	}

	/// `text` wrapped at its words into lines no wider than `max_width`, a word
	/// wider than a line broken where it overflows, and at most `max_lines`
	/// of them, the last ellipsized when the text runs on.
	fn wrap(&self, text: &str, style: TextStyle) -> Result<Vec<String>> {
		let fits = |line: &str| -> Result<bool> {
			(self.width(line, style.size, style.weight)? <= style.max_width)
				.xok()
		};
		let mut lines = Vec::<String>::new();
		let mut line = String::new();
		for word in text.split_whitespace() {
			let joined = match line.is_empty() {
				true => word.to_string(),
				false => format!("{line} {word}"),
			};
			if fits(&joined)? {
				line = joined;
				continue;
			}
			if !line.is_empty() {
				lines.push(core::mem::take(&mut line));
			}
			// a word wider than a whole line breaks where it overflows
			for ch in word.chars() {
				line.push(ch);
				if !fits(&line)? && line.chars().count() > 1 {
					line.pop();
					lines.push(core::mem::replace(&mut line, ch.to_string()));
				}
			}
		}
		if !line.is_empty() {
			lines.push(line);
		}
		if lines.len() > style.max_lines {
			lines.truncate(style.max_lines);
			let last = lines.last_mut().unwrap_or_else(|| unreachable!());
			loop {
				// a clause's punctuation reads wrong before the ellipsis
				let trimmed = last
					.trim_end_matches(|ch: char| {
						ch.is_whitespace()
							|| matches!(ch, ',' | ';' | ':' | '.')
					})
					.to_string();
				let ellipsized = format!("{trimmed}…");
				if fits(&ellipsized)? || trimmed.is_empty() {
					*last = ellipsized;
					break;
				}
				*last = trimmed;
				last.pop();
			}
		}
		lines.xok()
	}
}

/// How a block of card text is set and how much room it has.
#[cfg(feature = "site_images")]
#[derive(Debug, Clone, Copy)]
struct TextStyle {
	/// The font size in pixels.
	size: f32,
	/// The weight, the `wght` axis of a variable face.
	weight: f32,
	/// The distance between baselines.
	line_height: f32,
	/// The widest a line may be.
	max_width: f32,
	/// The most lines the block holds.
	max_lines: usize,
}

/// The card's fixed geometry: the constants every card shares.
#[cfg(feature = "site_images")]
struct CardLayout;

#[cfg(feature = "site_images")]
impl CardLayout {
	/// The card's width, the size every platform's large preview asks for.
	const WIDTH: f32 = 1200.;
	/// The card's height.
	const HEIGHT: f32 = 630.;
	/// The margin around everything.
	const MARGIN: f32 = 72.;
	/// The `Primary` band along the foot.
	const BAND: f32 = 14.;
	/// The logo's side, along the bottom.
	const LOGO: f32 = 72.;
	/// The media slot's left edge.
	const SLOT_X: f32 = 600.;
	/// The media slot's top edge.
	const SLOT_Y: f32 = Self::MARGIN;
	/// The media slot's width, 16:9 with its height so a video's frame fills
	/// it whole.
	const SLOT_WIDTH: u32 = 528;
	/// The media slot's height.
	const SLOT_HEIGHT: u32 = 297;
	/// The media slot's corner radius.
	const SLOT_RADIUS: f32 = 20.;
	/// The gap between the text column and the media slot.
	const GUTTER: f32 = 48.;
	/// The gap between the title and the description.
	const GAP: f32 = 20.;

	/// The text column's width, the whole card without media.
	fn column(media: bool) -> f32 {
		match media {
			true => Self::SLOT_X - Self::GUTTER - Self::MARGIN,
			false => Self::WIDTH - 2. * Self::MARGIN,
		}
	}

	/// The title's style.
	fn title(media: bool) -> TextStyle {
		TextStyle {
			size: 56.,
			weight: 700.,
			line_height: 66.,
			max_width: Self::column(media),
			max_lines: 3,
		}
	}

	/// The description's style.
	fn description(media: bool) -> TextStyle {
		TextStyle {
			size: 28.,
			weight: 400.,
			line_height: 38.,
			max_width: Self::column(media),
			max_lines: 2,
		}
	}
}

/// The tones a card is drawn in, the light scheme of the site's [`Theme`].
#[cfg(feature = "site_images")]
struct CardPalette {
	/// The frame.
	background: String,
	/// The band and the play badge.
	primary: String,
	/// The play badge's glyph.
	on_primary: String,
	/// The title and the site title.
	on_surface: String,
	/// The description and the host.
	on_surface_variant: String,
}

#[cfg(feature = "site_images")]
impl CardPalette {
	/// The light tones of `theme`.
	fn new(theme: &Theme) -> Result<Self> {
		let hex = |role: Token| -> Result<String> {
			theme
				.resolve(ColorScheme::Light, role)?
				.to_srgba()
				.to_hex()
				.xok()
		};
		use material::colors;
		Self {
			background: hex(colors::Background.into())?,
			primary: hex(colors::Primary.into())?,
			on_primary: hex(colors::OnPrimary.into())?,
			on_surface: hex(colors::OnSurface.into())?,
			on_surface_variant: hex(colors::OnSurfaceVariant.into())?,
		}
		.xok()
	}
}

/// A page's image fitted to the media slot.
#[cfg(feature = "site_images")]
struct CardMedia {
	/// The image centre-cropped and resized to the slot, as png.
	png: Vec<u8>,
	/// Whether it is a video's poster, which wears the play badge.
	video: bool,
}

#[cfg(feature = "site_images")]
impl CardMedia {
	/// `page`'s image in `bytes`, centre-cropped to the slot's aspect and
	/// resized to fill it.
	fn new(page: &CardPage, bytes: &[u8]) -> Result<Self> {
		let image = image::load_from_memory(bytes)
			.map_err(|err| {
				bevyhow!(
					"page `{}`'s image could not be decoded: {err}",
					page.path
				)
			})?
			.into_rgba8();
		let (width, height) = image.dimensions();
		let (slot_width, slot_height) =
			(CardLayout::SLOT_WIDTH, CardLayout::SLOT_HEIGHT);
		// the largest region of the slot's aspect the image holds, centred
		let (crop_width, crop_height) = match width as u64 * slot_height as u64
			> height as u64 * slot_width as u64
		{
			true => (
				(height as u64 * slot_width as u64 / slot_height as u64) as u32,
				height,
			),
			false => (
				width,
				(width as u64 * slot_height as u64 / slot_width as u64) as u32,
			),
		};
		let cropped = image::imageops::crop_imm(
			&image,
			(width - crop_width) / 2,
			(height - crop_height) / 2,
			crop_width.max(1),
			crop_height.max(1),
		)
		.to_image();
		let fitted = image::imageops::resize(
			&cropped,
			slot_width,
			slot_height,
			image::imageops::FilterType::Lanczos3,
		);
		Self {
			png: super::icon_set::IconSource::encode_png(&fitted)?,
			video: page.meta.image_url.is_none(),
		}
		.xok()
	}
}

/// One card laid out: its text in lines, ready to draw.
#[cfg(feature = "site_images")]
struct Card {
	/// The title's lines.
	title: Vec<String>,
	/// The description's lines.
	description: Vec<String>,
	/// The site title along the bottom.
	site: String,
	/// The homepage's host beneath it.
	host: Option<String>,
	/// The image filling the slot.
	media: Option<CardMedia>,
}

#[cfg(feature = "site_images")]
impl Card {
	/// `page`'s card, its text wrapped to the column `media` leaves it.
	fn new(
		font: &CardFont,
		package: &PackageConfig,
		page: &CardPage,
		media: Option<CardMedia>,
	) -> Result<Self> {
		let has_media = media.is_some();
		let title = page
			.meta
			.title
			.clone()
			.unwrap_or_else(|| package.title.to_string());
		let description = page
			.meta
			.description
			.clone()
			.unwrap_or_else(|| package.description.to_string());
		Self {
			title: font.wrap(&title, CardLayout::title(has_media))?,
			description: font
				.wrap(&description, CardLayout::description(has_media))?,
			site: package.title.to_string(),
			host: package
				.homepage
				.as_ref()
				.and_then(|homepage| homepage.host())
				.map(|host| host.trim_start_matches("www.").to_string()),
			media,
		}
		.xok()
	}

	/// The card as png: its svg rendered by `resvg`, the logo and the media
	/// resolved by name rather than by any path, so nothing is read from a
	/// filesystem.
	fn render(
		&self,
		font: &CardFont,
		palette: &CardPalette,
		logo: Option<&[u8]>,
	) -> Result<Vec<u8>> {
		use resvg::tiny_skia;
		use resvg::usvg;
		let svg = self.svg(font, palette, logo.is_some());
		let images = [
			("logo", logo.map(<[u8]>::to_vec)),
			("media", self.media.as_ref().map(|media| media.png.clone())),
		]
		.into_iter()
		.filter_map(|(name, png)| Some((name, std::sync::Arc::new(png?))))
		.collect::<Vec<_>>();
		let options = usvg::Options {
			fontdb: font.fontdb.clone(),
			font_family: font.family.clone(),
			image_href_resolver: usvg::ImageHrefResolver {
				resolve_data: Box::new(|_, _, _| None),
				resolve_string: Box::new(move |href, _| {
					images
						.iter()
						.find(|(name, _)| *name == href)
						.map(|(_, png)| usvg::ImageKind::PNG(png.clone()))
				}),
			},
			..default()
		};
		let tree = usvg::Tree::from_str(&svg, &options)?;
		let (width, height) =
			(CardLayout::WIDTH as u32, CardLayout::HEIGHT as u32);
		let mut pixmap = tiny_skia::Pixmap::new(width, height)
			.ok_or_else(|| bevyhow!("no {width}x{height} pixmap"))?;
		resvg::render(
			&tree,
			tiny_skia::Transform::default(),
			&mut pixmap.as_mut(),
		);
		pixmap.encode_png()?.xok()
	}

	/// The card as svg, its images named `logo` and `media`.
	fn svg(
		&self,
		font: &CardFont,
		palette: &CardPalette,
		logo: bool,
	) -> String {
		use core::fmt::Write;
		type Layout = CardLayout;
		let has_media = self.media.is_some();
		let (width, height) = (Layout::WIDTH, Layout::HEIGHT);
		let family = escape(&font.family);
		let mut svg = format!(
			r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
		);
		// the frame: the background and the band along the foot
		let _ = write!(
			svg,
			r#"<rect width="{width}" height="{height}" fill="{}"/><rect y="{}" width="{width}" height="{}" fill="{}"/>"#,
			palette.background,
			height - Layout::BAND,
			Layout::BAND,
			palette.primary,
		);
		// the media slot, the image clipped to its rounded corners
		if let Some(media) = &self.media {
			let (x, y) = (Layout::SLOT_X, Layout::SLOT_Y);
			let (slot_width, slot_height) =
				(Layout::SLOT_WIDTH, Layout::SLOT_HEIGHT);
			let _ = write!(
				svg,
				r#"<clipPath id="slot"><rect x="{x}" y="{y}" width="{slot_width}" height="{slot_height}" rx="{}"/></clipPath><image xlink:href="media" x="{x}" y="{y}" width="{slot_width}" height="{slot_height}" preserveAspectRatio="none" clip-path="url(#slot)"/>"#,
				Layout::SLOT_RADIUS,
			);
			if media.video {
				let (cx, cy) =
					(x + slot_width as f32 / 2., y + slot_height as f32 / 2.);
				let _ = write!(
					svg,
					r#"<circle cx="{cx}" cy="{cy}" r="44" fill="{}" fill-opacity="0.92"/><path d="M{} {}L{} {}L{} {}Z" fill="{}"/>"#,
					palette.primary,
					cx - 14.,
					cy - 22.,
					cx + 24.,
					cy,
					cx - 14.,
					cy + 22.,
					palette.on_primary,
				);
			}
		}
		// the title, then the description beneath it
		let text = |svg: &mut String,
		            lines: &[String],
		            style: TextStyle,
		            top: f32,
		            fill: &str|
		 -> f32 {
			let mut baseline = top;
			for line in lines {
				baseline += style.line_height;
				let _ = write!(
					svg,
					r#"<text x="{}" y="{baseline}" font-family="{family}" font-size="{}" font-weight="{}" fill="{fill}">{}</text>"#,
					Layout::MARGIN,
					style.size,
					style.weight,
					escape(line),
				);
			}
			baseline
		};
		// the first baseline sits a line below the margin, less the leading
		let title = Layout::title(has_media);
		let top = Layout::MARGIN - (title.line_height - title.size);
		let after_title =
			text(&mut svg, &self.title, title, top, &palette.on_surface);
		text(
			&mut svg,
			&self.description,
			Layout::description(has_media),
			after_title + Layout::GAP,
			&palette.on_surface_variant,
		);
		// the bottom row: the logo, the site title and the host
		let row = height - Layout::MARGIN - Layout::LOGO;
		let mut x = Layout::MARGIN;
		if logo {
			let _ = write!(
				svg,
				r#"<image xlink:href="logo" x="{x}" y="{row}" width="{side}" height="{side}"/>"#,
				side = Layout::LOGO,
			);
			x += Layout::LOGO + 24.;
		}
		let (site_baseline, host_baseline) = match &self.host {
			Some(_) => (row + 32., row + Layout::LOGO - 4.),
			None => (row + Layout::LOGO / 2. + 11., 0.),
		};
		let _ = write!(
			svg,
			r#"<text x="{x}" y="{site_baseline}" font-family="{family}" font-size="30" font-weight="700" fill="{}">{}</text>"#,
			palette.on_surface,
			escape(&self.site),
		);
		if let Some(host) = &self.host {
			let _ = write!(
				svg,
				r#"<text x="{x}" y="{host_baseline}" font-family="{family}" font-size="24" font-weight="400" fill="{}">{}</text>"#,
				palette.on_surface_variant,
				escape(host),
			);
		}
		svg.push_str("</svg>");
		svg
	}
}

#[cfg(all(test, feature = "site_images"))]
mod test {
	use super::*;
	use beet_action::prelude::*;

	/// A Latin subset of Inter Bold, the font every test card is set in.
	const FONT: &[u8] = include_bytes!("fixtures/inter-bold-latin.ttf");
	/// Where the test store holds [`FONT`].
	const FONT_PATH: &str = "fonts/inter.ttf";

	/// A `width` by `height` png of one `color`.
	fn png(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
		super::super::icon_set::IconSource::encode_png(
			&image::RgbaImage::from_pixel(width, height, image::Rgba(color)),
		)
		.unwrap()
	}

	/// A post's markdown declaring `title` and `extra` frontmatter.
	fn post(title: &str, extra: &str) -> String {
		format!(
			"+++\ntitle = \"{title}\"\ndescription = \"about {title}\"\n{extra}\n+++\n\n# {title}"
		)
	}

	/// A world whose `PackageConfig` names the site, the reactions running
	/// so an edited file refreshes its route.
	fn world() -> World {
		let mut world =
			(MinimalPlugins, AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(PackageConfig {
			title: "Beet".into(),
			description: "An Atmospheric OS for homegrown tech".into(),
			homepage: Some(Url::coerce("https://beet.org")),
			..default()
		});
		world
	}

	/// A router over the store's `routes` beside a set of cards, the store
	/// holding the font and `files`.
	async fn setup(files: &[(&str, Vec<u8>)]) -> (World, Entity, BlobStore) {
		let inner = InMemoryStore::new();
		let store = BlobStore::new(inner.clone());
		store
			.insert(&RelPath::new(FONT_PATH), FONT.to_vec())
			.await
			.unwrap();
		for (path, bytes) in files {
			store
				.insert(&RelPath::new(*path), bytes.clone())
				.await
				.unwrap();
		}
		let mut world = world();
		world
			.spawn((inner, Router, children![
				RoutesDir::new("routes"),
				SocialCards::new(FONT_PATH),
			]))
			.flush();
		settle(&mut world).await;
		let cards = world
			.query_filtered::<Entity, With<SocialCards>>()
			.single(&world)
			.unwrap();
		(world, cards, store)
	}

	/// Run a frame and settle the tasks it spawned, as a boot does before it
	/// serves.
	async fn settle(world: &mut World) {
		world.update_local();
		AsyncRunner::settle_async_tasks(world).await;
	}

	/// [`Derivation::ensure`] for the set in `world`'s async context.
	async fn ensure(
		world: &mut World,
		cards: Entity,
		store: &BlobStore,
	) -> Freshness {
		let store = store.clone();
		world
			.run_async_then(async move |world| {
				Derivation::ensure(&world, cards, &store).await
			})
			.await
			.unwrap()
	}

	/// The cards the set's document lists.
	async fn outputs(store: &BlobStore) -> Vec<String> {
		SocialCards::new(FONT_PATH)
			.derivation()
			.read(store)
			.await
			.unwrap()
			.map(|body| DerivationOutputs::of(&body).unwrap())
			.unwrap()
			.0
			.into_iter()
			.map(|output| output.path.to_string())
			.collect()
	}

	/// The card at `path` in `store`.
	async fn card(store: &BlobStore, path: &str) -> image::RgbaImage {
		image::load_from_memory(&store.get(&RelPath::new(path)).await.unwrap())
			.unwrap()
			.into_rgba8()
	}

	/// Two pages, the root and a post.
	fn two_pages() -> [(&'static str, Vec<u8>); 2] {
		[
			("routes/index.md", post("Beet", "").into_bytes()),
			("routes/blog/post.md", post("A Post", "").into_bytes()),
		]
	}

	#[beet_core::test]
	async fn derives_the_set_idempotently() {
		let (mut world, cards, store) = setup(&two_pages()).await;
		ensure(&mut world, cards, &store)
			.await
			.xpect_eq(Freshness::Missing);
		outputs(&store)
			.await
			.xpect_eq(vec!["assets/og/index.png", "assets/og/blog/post.png"]);
		card(&store, "assets/og/blog/post.png")
			.await
			.dimensions()
			.xpect_eq((1200, 630));
		ensure(&mut world, cards, &store)
			.await
			.xpect_eq(Freshness::Fresh);
	}

	#[beet_core::test]
	async fn regenerates_a_changed_page() {
		let (mut world, cards, store) = setup(&two_pages()).await;
		ensure(&mut world, cards, &store).await;
		let before = store
			.get(&RelPath::new("assets/og/blog/post.png"))
			.await
			.unwrap();
		store
			.insert(
				&RelPath::new("routes/blog/post.md"),
				post("A Renamed Post", ""),
			)
			.await
			.unwrap();
		settle(&mut world).await;
		ensure(&mut world, cards, &store)
			.await
			.to_string()
			.xpect_eq("value `assets/og/blog/post.png` changed");
		(store
			.get(&RelPath::new("assets/og/blog/post.png"))
			.await
			.unwrap() != before)
			.xpect_true();
	}

	/// A page declared in code has no file to track, so its drawn text is a
	/// value, as is the frame a rebrand redraws.
	#[beet_core::test]
	async fn regenerates_a_page_declared_in_code() {
		let (mut world, cards, store) = setup(&two_pages()).await;
		let router = world.entity(cards).get::<ChildOf>().unwrap().parent();
		let about = |title: &str| PageMeta {
			title: Some(title.into()),
			..default()
		};
		let page = world
			.spawn((ChildOf(router), page("about", about("About"))))
			.id();
		settle(&mut world).await;
		ensure(&mut world, cards, &store).await;
		outputs(&store)
			.await
			.contains(&"assets/og/about.png".to_string())
			.xpect_true();
		ensure(&mut world, cards, &store)
			.await
			.xpect_eq(Freshness::Fresh);
		world.entity_mut(page).insert(about("About Beet"));
		settle(&mut world).await;
		ensure(&mut world, cards, &store)
			.await
			.xpect_eq(Freshness::Revalued("assets/og/about.png".into()));
		world.insert_resource(Theme {
			color: Color::srgb(0.5, 0.2, 0.8),
			..default()
		});
		settle(&mut world).await;
		ensure(&mut world, cards, &store)
			.await
			.xpect_eq(Freshness::Revalued("frame".into()));
	}

	/// A page added since the set was derived is a source the document was
	/// not computed from, and a page removed takes its card with it.
	#[beet_core::test]
	async fn tracks_added_and_removed_pages() {
		let (mut world, cards, store) = setup(&two_pages()).await;
		ensure(&mut world, cards, &store).await;
		store
			.insert(&RelPath::new("routes/blog/next.md"), post("Next", ""))
			.await
			.unwrap();
		settle(&mut world).await;
		ensure(&mut world, cards, &store).await.xpect_eq(
			Freshness::Redeclared(RelPath::new("routes/blog/next.md")),
		);
		outputs(&store)
			.await
			.contains(&"assets/og/blog/next.png".to_string())
			.xpect_true();
		store
			.remove(&RelPath::new("routes/blog/post.md"))
			.await
			.unwrap();
		settle(&mut world).await;
		ensure(&mut world, cards, &store)
			.await
			.is_fresh()
			.xpect_false();
		outputs(&store)
			.await
			.xpect_eq(vec!["assets/og/index.png", "assets/og/blog/next.png"]);
		store
			.exists(&RelPath::new("assets/og/blog/post.png"))
			.await
			.unwrap()
			.xpect_false();
	}

	/// A page's store image fills the slot, the frame around it untouched, and
	/// a changed image regenerates the set.
	#[beet_core::test]
	async fn embeds_a_store_image() {
		const RED: [u8; 4] = [255, 0, 0, 255];
		let (mut world, cards, store) = setup(&[
			(
				"routes/post.md",
				post("Pictured", "image_url = \"/media/cover.png\"")
					.into_bytes(),
			),
			("media/cover.png", png(800, 450, RED)),
		])
		.await;
		ensure(&mut world, cards, &store).await;
		let drawn = card(&store, "assets/og/post.png").await;
		let slot_centre = (
			CardLayout::SLOT_X as u32 + CardLayout::SLOT_WIDTH / 2,
			CardLayout::SLOT_Y as u32 + CardLayout::SLOT_HEIGHT / 2,
		);
		drawn
			.get_pixel(slot_centre.0, slot_centre.1)
			.0
			.xpect_eq(RED);
		// below the slot and right of the text column, the frame alone
		let background = Theme::default()
			.resolve(ColorScheme::Light, material::colors::Background)
			.unwrap()
			.to_srgba()
			.to_u8_array();
		drawn.get_pixel(1150, 450).0.xpect_eq(background);
		store
			.insert(
				&RelPath::new("media/cover.png"),
				png(800, 450, [0, 0, 255, 255]),
			)
			.await
			.unwrap();
		ensure(&mut world, cards, &store)
			.await
			.to_string()
			.xpect_eq("file `media/cover.png` changed");
		card(&store, "assets/og/post.png")
			.await
			.get_pixel(slot_centre.0, slot_centre.1)
			.0
			.xpect_eq([0, 0, 255, 255]);
	}

	/// A remote poster is fetched while the set derives, and a rerun is fresh
	/// without fetching it again.
	#[cfg(all(feature = "http", not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn fetches_a_remote_poster_once() {
		use std::sync::atomic::AtomicUsize;
		use std::sync::atomic::Ordering;
		let fetches = std::sync::Arc::new(AtomicUsize::new(0));
		let (mut server, on_spawn) =
			HttpServer::new_test(HttpServer::start_mini_with_tcp);
		// leave the process-global loopback port to whoever owns it
		server.canonical = false;
		let poster = format!("{}/poster.png", server.local_url());
		let counted = fetches.clone();
		std::thread::spawn(move || {
			App::new()
				.add_plugins((MinimalPlugins, RouterPlugin))
				.spawn((server, on_spawn, children![(Router, children![
					route::exchange(
						"poster.png",
						Action::<Request, Response>::new_async(
							move |_: ActionContext<Request>| {
								counted.fetch_add(1, Ordering::SeqCst);
								async {
									Response::ok_body(
										png(480, 360, [0, 255, 0, 255]),
										MediaType::Png,
									)
									.xok()
								}
							}
						)
					)
				])]))
				.run();
		});
		let (mut world, cards, store) = setup(&[(
			"routes/post.md",
			post("Remote", &format!("image_url = \"{poster}\"")).into_bytes(),
		)])
		.await;
		ensure(&mut world, cards, &store)
			.await
			.xpect_eq(Freshness::Missing);
		fetches.load(Ordering::SeqCst).xpect_eq(1);
		ensure(&mut world, cards, &store)
			.await
			.xpect_eq(Freshness::Fresh);
		fetches.load(Ordering::SeqCst).xpect_eq(1);
	}

	/// An image that cannot be read fails the set naming the page, never a
	/// card silently without its media.
	#[beet_core::test]
	async fn refuses_a_missing_image() {
		let (mut world, cards, store) = setup(&[(
			"routes/post.md",
			post("Broken", "image_url = \"/media/missing.png\"").into_bytes(),
		)])
		.await;
		let ensured = store.clone();
		world
			.run_async_then(async move |world| {
				Derivation::ensure(&world, cards, &ensured).await
			})
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("page `post`'s image `/media/missing.png`");
	}

	/// A long title wraps to three lines inside the column, the last
	/// ellipsized.
	#[beet_core::test]
	fn wraps_a_long_title() {
		let font = CardFont::new(FONT.to_vec()).unwrap();
		for media in [false, true] {
			let style = CardLayout::title(media);
			let lines = font
				.wrap(
					&"Gentle slopes up lonely mountains and the long road home ".repeat(4),
					style,
				)
				.unwrap();
			lines.len().xpect_eq(3);
			lines[2].ends_with('…').xpect_true();
			lines[2].ends_with(" …").xpect_false();
			for line in &lines {
				let width = font.width(line, style.size, style.weight).unwrap();
				(width <= style.max_width).xpect_true();
				(CardLayout::MARGIN + width
					<= CardLayout::WIDTH - CardLayout::MARGIN)
					.xpect_true();
			}
		}
		// a word wider than a line breaks rather than overflowing
		let style = CardLayout::title(true);
		font.wrap(&"W".repeat(40), style)
			.unwrap()
			.iter()
			.all(|line| {
				font.width(line, style.size, style.weight).unwrap()
					<= style.max_width
			})
			.xpect_true();
	}
}

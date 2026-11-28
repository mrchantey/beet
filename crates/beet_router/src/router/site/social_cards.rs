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
//! the page's [`DocMeta::credit_line`](beet_ui::prelude::DocMeta::credit_line)
//! as an eyebrow in the `Primary` tone (a page with neither author nor date
//! wears none, and one too long for a line breaks between its authors and its
//! date), the description in the `OnSurfaceVariant` tone (filling the room
//! left, then ellipsized), and along the bottom the logo, the site title and the homepage's
//! host. The logo is the source image of the world's one [`IconSet`], so a
//! site declares it once.
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
//! world (the page's title, credit line, description and media url), and
//! names the two resources the frame is drawn from, the theme's [`ThemeTones`]
//! and the [`PackageConfig`] (the site title, description and homepage),
//! read as the set's freshness is checked, so a page declared in code or a
//! rebrand regenerates the set like an edited file ([`ProvenanceValue`]). A page added is a value the document was not
//! computed from and a page removed one it no longer declares, so both
//! regenerate too, and the removed page's card is pruned as a superseded
//! output. One document rather than one per page: a regeneration redraws
//! every card, which costs little at a site's size.
//!
//! The layout itself is code, which no source or value names: after changing
//! it, remove `assets/og/social-cards.json` so the next derivation redraws the
//! set.
//!
//! # The head
//!
//! [`RouteHead`] hands `<Head>` the card's absolute url as the page's social
//! image, ahead of the page's own image, which a site without `<SocialCards/>`
//! still previews with. A page outside the set (unlisted, say) previews with
//! the root route's card, so every page of a site with cards previews as a
//! card. The card's text goes with it as the image's alt text, which an
//! announcement's thumb cannot carry: a Bluesky link card has no field for one.
//! The standard.site `coverImage` stays the page's raw image, since a
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
	font: FwdPath,
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
	const DIR: FwdPath = fwd_path!("assets/og");
	/// The derived document listing the set.
	const DOCUMENT: FwdPath = fwd_path!("social-cards.json");

	/// The cards set in `font`.
	pub fn new(font: impl Into<FwdPath>) -> Self { Self { font: font.into() } }

	/// The [`Derivation`] this set declares, its document in `assets/og`
	/// naming the font and the resources the frame is drawn from; the logo,
	/// the pages and their values join it as the world names them
	/// ([`refresh_inputs`](Self::refresh_inputs)).
	pub fn derivation(&self) -> Derivation {
		Derivation::new(Self::DIR.join(&Self::DOCUMENT))
			.with_input(self.font.clone())
			.with_resource_type::<ThemeTones>()
			.with_resource_type::<PackageConfig>()
	}

	/// The store path of the card for the page at route `path`: the route's
	/// path with `.png` appended below [`DIR`](Self::DIR), the root's
	/// `index.png`.
	pub fn card(path: &FwdPath) -> FwdPath {
		match path.split_last() {
			Some((parent, last)) => Self::DIR
				.join(&parent)
				.join_segment(&path_segment!("{}.png", last)),
			None => Self::DIR.join_segment(&path_segment!("index.png")),
		}
	}

	/// The page whose card a page at route `route` previews with: its own when
	/// `listed` holds it, else the root route's, else none.
	fn previewed(
		listed: &[SyndicationPage],
		route: Entity,
	) -> Option<&SyndicationPage> {
		listed.iter().find(|page| page.route == route).or_else(|| {
			listed.iter().find(|page| page.path.as_str().is_empty())
		})
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
	/// something a card names lands: a route tree, a page's metadata or the
	/// logo. The resources the frame is drawn from need none, being read as
	/// the set's freshness is checked.
	fn install(app: &mut App) {
		app.register_type::<Self>()
			.add_observer(queue_refresh::<RouteTree>)
			.add_observer(queue_refresh::<DocMeta>)
			.add_observer(queue_refresh::<IconSet>);
	}
}

/// Observer queueing [`SocialCards::refresh_inputs`] once `T` lands, a
/// resource included, so the refresh reads the world whole.
fn queue_refresh<T: Component>(_ev: On<Insert<T>>, mut commands: Commands) {
	commands.queue(SocialCards::refresh_inputs);
}

/// The card the page at `route` previews with, when the world
/// declares a [`SocialCards`] set: see [`RouteHead`].
#[derive(SystemParam)]
pub(crate) struct SocialCardQuery<'w, 's> {
	cards: Query<'w, 's, Entity, With<SocialCards>>,
	syndication: SyndicationQuery<'w, 's>,
}

impl SocialCardQuery<'_, '_> {
	/// The absolute url of `route`'s card and its alt text, `None` without a
	/// set, outside its url space or with no `homepage` to resolve against,
	/// since a crawler needs an absolute `og:image`.
	///
	/// # Errors
	/// Errors when the world declares more than one set.
	pub fn preview(&self, route: Entity) -> Result<Option<(Url, String)>> {
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
		let package = self.syndication.package();
		SocialCards::previewed(&listed, route)
			.and_then(|page| {
				let href = IconSet::href(&SocialCards::card(&page.path));
				let url = package.absolute_url(&href).ok()?;
				Some((url, CardText::new(&page.meta, package).alt()))
			})
			.xok()
	}
}

/// The words a page's card draws, the site's own where the page declares none.
struct CardText {
	/// The title.
	title: String,
	/// The page's [`DocMeta::credits`].
	credits: Vec<String>,
	/// The description.
	description: String,
}

impl CardText {
	/// The words on the card of a page declaring `meta` in the site `package`.
	fn new(meta: &DocMeta, package: &PackageConfig) -> Self {
		Self {
			title: meta
				.title
				.clone()
				.unwrap_or_else(|| package.title.to_string()),
			credits: meta.credits(),
			description: meta
				.description
				.clone()
				.unwrap_or_else(|| package.description.to_string()),
		}
	}

	/// The card's alt text: its words in reading order, a paragraph per block
	/// and the credits in the case they were authored in, since a screen
	/// reader may spell out capitals.
	fn alt(&self) -> String {
		[
			self.title.as_str(),
			&self.credits.join(", "),
			&self.description,
		]
		.into_iter()
		.filter(|block| !block.is_empty())
		.map(|block| match block.ends_with(['.', '!', '?', '…']) {
			true => block.to_string(),
			false => format!("{block}."),
		})
		.collect::<Vec<_>>()
		.join(" ")
	}
}

/// What a [`SocialCards`] set reads off the world: its pages, each with the
/// file it was discovered from, and the logo.
#[derive(SystemParam)]
struct SocialCardsQuery<'w, 's> {
	syndication: SyndicationQuery<'w, 's>,
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
	) -> Result<Vec<(SyndicationPage, Option<FwdPath>)>> {
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
	fn inputs(&self, cards: Entity) -> Vec<FwdPath> {
		let logo = self.logo().ok().flatten().map(|logo| logo.src);
		let sources = self
			.pages(cards)
			.into_iter()
			.flatten()
			.filter_map(|(_, source)| source);
		logo.into_iter().chain(sources).collect()
	}

	/// The values the set at `cards` draws off the world, as far as it can
	/// name them now: each card's text and media url, named by the card's
	/// path. A value is the drawn fields joined, so it changes exactly when
	/// the drawing could.
	fn values(&self, cards: Entity) -> Vec<ProvenanceValue> {
		self.pages(cards)
			.into_iter()
			.flatten()
			.map(|(page, _)| {
				let card = SocialCards::card(&page.path);
				// a poster url is only ever a video's, so the url implies the badge
				let media =
					page.meta.social_image_url().map(|url| url.to_string());
				let credit = page.meta.credit_line();
				Self::value(card.as_str(), [
					page.meta.title.as_deref(),
					credit.as_deref(),
					page.meta.description.as_deref(),
					media.as_deref(),
				])
			})
			.collect()
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
	) -> Result<Option<FwdPath>> {
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
		subdir.join(blob.path()).xmap(Some).xok()
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
	/// The tones the frame is drawn in.
	tones: ThemeTones,
	/// The site title, the homepage and the fallback description.
	package: PackageConfig,
}

/// One page a card is drawn for.
#[cfg(feature = "site_images")]
struct CardPage {
	/// The route path, ie `blog/ecs-router`.
	path: FwdPath,
	/// The page's authored metadata.
	meta: DocMeta,
	/// The page's source file in the store the cards are derived in, `None`
	/// for a page declared in code.
	source: Option<FwdPath>,
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
			tones: world
				.get_resource::<ThemeTones>()
				.cloned()
				.unwrap_or_default(),
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
		// read every source first, so the drawing needs no store
		let mut sources = CardSources::default();
		let font = sources.file(store, &self.font).await?;
		let logo = match &set.logo {
			Some(icon_set) => Some(sources.file(store, &icon_set.src).await?),
			None => None,
		};
		let mut media = Vec::with_capacity(set.pages.len());
		for page in &set.pages {
			if let Some(source) = &page.source {
				sources.file(store, source).await?;
			}
			media.push(match page.meta.social_image_url() {
				Some(url) => Some(sources.media(store, page, url).await?),
				None => None,
			});
		}
		let cards =
			async_ext::unblock(move || set.draw(font, logo, media)).await?;
		let mut outputs = DerivationOutputs::default();
		for (path, card) in cards {
			outputs.write(store, path, card).await?;
		}
		EntityBody::default()
			.with_typed(&Provenance::new(sources.0))?
			.with_typed(&outputs)
	}
}

#[cfg(feature = "site_images")]
impl CardSet {
	/// Every card of the set as png, by its store path: set in the `font`
	/// file, wearing the `logo` source, each page's image its bytes in
	/// `media`, in page order. CPU bound, so [`SocialCards::derive`] runs it
	/// off the world's thread.
	fn draw(
		self,
		font: Vec<u8>,
		logo: Option<Vec<u8>>,
		media: Vec<Option<Vec<u8>>>,
	) -> Result<Vec<(FwdPath, Vec<u8>)>> {
		let font = CardFont::new(font)?;
		let logo = self
			.logo
			.as_ref()
			.zip(logo)
			.map(|(icon_set, bytes)| {
				super::icon_set::IconSource::decode(icon_set, &bytes)?
					.render(CardLayout::LOGO as u32)?
					.xmap(|logo| super::icon_set::IconSource::encode_png(&logo))
			})
			.transpose()?;
		let palette = CardPalette::new(&self.tones)?;
		self.pages
			.iter()
			.zip(media)
			.map(|(page, media)| {
				let media = media
					.map(|bytes| CardMedia::new(page, &bytes))
					.transpose()?;
				let card = Card::new(&font, &self.package, page, media)?
					.render(&font, &palette, logo.as_deref())?;
				(SocialCards::card(&page.path), card).xok()
			})
			.collect()
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
		path: &FwdPath,
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
				true => url.fwd_path()?,
				// a page-relative image resolves beside the page, as a
				// browser resolves it against the page's url, never above
				// the site's root
				false => page
					.path
					.parent()
					.unwrap_or_default()
					.as_smol_path()
					.join(url.path().join("/"))
					.xmap(FwdPath::new)?,
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

	/// The advance width of `text` set in `style`, at the optical size `usvg`
	/// picks for its size, as it renders.
	fn width(&self, text: &str, style: TextStyle) -> Result<f32> {
		use skrifa::MetadataProvider;
		let font = skrifa::FontRef::new(&self.bytes)?;
		let location = font
			.axes()
			.location([("wght", style.weight), ("opsz", style.size)]);
		let metrics = font
			.glyph_metrics(skrifa::instance::Size::new(style.size), &location);
		let charmap = font.charmap();
		text.chars()
			.filter_map(|ch| charmap.map(ch))
			.filter_map(|glyph| metrics.advance_width(glyph))
			.map(|advance| advance + style.tracking)
			.sum::<f32>()
			.xok()
	}

	/// `credits` as the uppercase credit line, broken between the authors and
	/// the date when one line cannot hold both.
	fn wrap_credits(
		&self,
		credits: &[String],
		style: TextStyle,
	) -> Result<Vec<String>> {
		let line = credits.join(" · ").to_uppercase();
		if self.width(&line, style)? <= style.max_width {
			return self.wrap(&line, style);
		}
		credits
			.iter()
			.map(|credit| self.wrap(&credit.to_uppercase(), style))
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>()
			.xok()
	}

	/// `text` wrapped at its words into lines no wider than `max_width`, a word
	/// wider than a line broken where it overflows, and at most `max_lines`
	/// of them, the last ellipsized when the text runs on.
	fn wrap(&self, text: &str, style: TextStyle) -> Result<Vec<String>> {
		let fits = |line: &str| -> Result<bool> {
			(self.width(line, style)? <= style.max_width).xok()
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
	/// The space added after each glyph.
	tracking: f32,
	/// The distance between baselines.
	line_height: f32,
	/// The widest a line may be.
	max_width: f32,
	/// The most lines the block holds.
	max_lines: usize,
}

#[cfg(feature = "site_images")]
impl TextStyle {
	/// The last baseline of `lines` lines set `gap` beneath the baseline
	/// `above`, a block of none taking no room.
	fn end(&self, above: f32, gap: f32, lines: usize) -> f32 {
		match lines {
			0 => above,
			lines => above + gap + self.line_height * lines as f32,
		}
	}

	/// Hold as many lines as fit `gap` beneath the baseline `above` with the
	/// last no lower than `floor`, always one.
	fn filling(self, above: f32, gap: f32, floor: f32) -> Self {
		let room = (floor - above - gap) / self.line_height;
		Self {
			max_lines: (room.floor() as usize).max(1),
			..self
		}
	}
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
	/// The gap above the description.
	const GAP: f32 = 20.;
	/// The gap above the credit line, tighter since it belongs to the title.
	const CREDIT_GAP: f32 = 18.;
	/// The top edge of the bottom row.
	const ROW: f32 = Self::HEIGHT - Self::MARGIN - Self::LOGO;
	/// The lowest baseline the text may reach, clear of the bottom row.
	const FLOOR: f32 = Self::ROW - 24.;

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
			tracking: 0.,
			line_height: 66.,
			max_width: Self::column(media),
			max_lines: 3,
		}
	}

	/// The baseline the title hangs from, its first line's cap at the margin.
	fn top(media: bool) -> f32 {
		let title = Self::title(media);
		Self::MARGIN - (title.line_height - title.size)
	}

	/// The credit line's style, an eyebrow.
	fn credit(media: bool) -> TextStyle {
		TextStyle {
			size: 22.,
			weight: 700.,
			tracking: 1.5,
			line_height: 30.,
			max_width: Self::column(media),
			max_lines: 2,
		}
	}

	/// The description's style, filling the room beneath the baseline `above`.
	fn description(media: bool, above: f32) -> TextStyle {
		TextStyle {
			size: 28.,
			weight: 400.,
			tracking: 0.,
			line_height: 38.,
			max_width: Self::column(media),
			max_lines: 1,
		}
		.filling(above, Self::GAP, Self::FLOOR)
	}
}

/// The tones a card is drawn in, the light scheme of the site's [`ThemeTones`].
#[cfg(feature = "site_images")]
struct CardPalette {
	/// The frame.
	background: String,
	/// The band, the credit line and the play badge.
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
	/// The light tones of `tones`.
	fn new(tones: &ThemeTones) -> Result<Self> {
		let hex = |role: Token| -> Result<String> {
			tones
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
	/// The credit line's lines, none for a page without one.
	credit: Vec<String>,
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
		let text = CardText::new(&page.meta, package);
		let title_style = CardLayout::title(has_media);
		let title = font.wrap(&text.title, title_style)?;
		let credit_style = CardLayout::credit(has_media);
		let credit = font.wrap_credits(&text.credits, credit_style)?;
		// the description takes the room the blocks above it leave
		let after_title =
			title_style.end(CardLayout::top(has_media), 0., title.len());
		let after_credit =
			credit_style.end(after_title, CardLayout::CREDIT_GAP, credit.len());
		let description = font.wrap(
			&text.description,
			CardLayout::description(has_media, after_credit),
		)?;
		Self {
			title,
			credit,
			description,
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
		// the title, the credit line, then the description, each `gap` beneath
		// the block above and an empty one taking no room
		let text = |svg: &mut String,
		            lines: &[String],
		            style: TextStyle,
		            above: f32,
		            gap: f32,
		            fill: &str|
		 -> f32 {
			for (i, line) in lines.iter().enumerate() {
				let baseline = style.end(above, gap, i + 1);
				let _ = write!(
					svg,
					r#"<text x="{}" y="{baseline}" font-family="{family}" font-size="{}" font-weight="{}" letter-spacing="{}" fill="{fill}">{}</text>"#,
					Layout::MARGIN,
					style.size,
					style.weight,
					style.tracking,
					escape(line),
				);
			}
			style.end(above, gap, lines.len())
		};
		let after_title = text(
			&mut svg,
			&self.title,
			Layout::title(has_media),
			Layout::top(has_media),
			0.,
			&palette.on_surface,
		);
		let after_credit = text(
			&mut svg,
			&self.credit,
			Layout::credit(has_media),
			after_title,
			Layout::CREDIT_GAP,
			&palette.primary,
		);
		text(
			&mut svg,
			&self.description,
			Layout::description(has_media, after_credit),
			after_credit,
			Layout::GAP,
			&palette.on_surface_variant,
		);
		// the bottom row: the logo, the site title and the host
		let row = Layout::ROW;
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

	/// A Latin subset of Inter Bold, the font every test card is set in.
	const FONT: &[u8] = include_bytes!("fixtures/inter-bold-latin.ttf");
	/// Where the test store holds [`FONT`].
	const FONT_PATH: FwdPath = fwd_path!("fonts/inter.ttf");

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
		let mut world = (
			MinimalPlugins,
			AsyncPlugin,
			RouterPlugin,
			material::MaterialStylePlugin,
		)
			.into_world();
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
		let inner = InMemoryBlobStore::new();
		let store = BlobStore::new(inner.clone());
		store
			.insert(&FwdPath::new(FONT_PATH).unwrap(), FONT.to_vec())
			.await
			.unwrap();
		for (path, bytes) in files {
			store
				.insert(&FwdPath::new(*path).unwrap(), bytes.clone())
				.await
				.unwrap();
		}
		let mut world = world();
		world
			.spawn((inner, Router, children![
				RoutesDir::new(fwd_path!("routes")),
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
		image::load_from_memory(&store.get(&FwdPath::new(path).unwrap()).await.unwrap())
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
			.get(&fwd_path!("assets/og/blog/post.png"))
			.await
			.unwrap();
		store
			.insert(
				&fwd_path!("routes/blog/post.md"),
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
			.get(&fwd_path!("assets/og/blog/post.png"))
			.await
			.unwrap() != before)
			.xpect_true();
	}

	/// A page declared in code has no file to track, so its drawn text is a
	/// value, and a rebrand redraws the frame from the tones it reads.
	#[beet_core::test]
	async fn regenerates_a_page_declared_in_code() {
		let (mut world, cards, store) = setup(&two_pages()).await;
		let router = world.entity(cards).get::<ChildOf>().unwrap().parent();
		let about = |title: &str| DocMeta {
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
			.xpect_eq(Freshness::Revalued(ThemeTones::type_path().into()));
	}

	/// A page added since the set was derived is a source the document was
	/// not computed from, and a page removed takes its card with it.
	#[beet_core::test]
	async fn tracks_added_and_removed_pages() {
		let (mut world, cards, store) = setup(&two_pages()).await;
		ensure(&mut world, cards, &store).await;
		store
			.insert(&fwd_path!("routes/blog/next.md"), post("Next", ""))
			.await
			.unwrap();
		settle(&mut world).await;
		ensure(&mut world, cards, &store).await.xpect_eq(
			Freshness::Redeclared(fwd_path!("routes/blog/next.md")),
		);
		outputs(&store)
			.await
			.contains(&"assets/og/blog/next.png".to_string())
			.xpect_true();
		store
			.remove(&fwd_path!("routes/blog/post.md"))
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
			.exists(&fwd_path!("assets/og/blog/post.png"))
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
		let background = ThemeTones::default()
			.resolve(ColorScheme::Light, material::colors::Background)
			.unwrap()
			.to_srgba()
			.to_u8_array();
		drawn.get_pixel(1150, 450).0.xpect_eq(background);
		store
			.insert(
				&fwd_path!("media/cover.png"),
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
		use beet_action::prelude::*;
		use std::sync::atomic::AtomicUsize;
		use std::sync::atomic::Ordering;
		let fetches = std::sync::Arc::new(AtomicUsize::new(0));
		let (mut server, on_spawn) =
			HttpServer::bind_loopback(HttpServer::start_mini_with_tcp);
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

	/// The credit line sits between the title and the description beside the
	/// media slot, its date never ellipsized, and a page with neither author
	/// nor date gives it no room.
	#[beet_core::test]
	fn credits_the_page() {
		let font = CardFont::new(FONT.to_vec()).unwrap();
		let palette = CardPalette::new(&ThemeTones::default()).unwrap();
		let svg = |meta: DocMeta, media: Option<CardMedia>| {
			let page = CardPage {
				path: fwd_path!("post"),
				meta: DocMeta {
					title: Some("A Post".into()),
					description: Some("about it".into()),
					..meta
				},
				source: None,
			};
			let card =
				Card::new(&font, &PackageConfig::default(), &page, media)
					.unwrap();
			(card.credit.clone(), card.svg(&font, &palette, false))
		};
		let baseline = |svg: &str, text: &str| {
			let line = svg
				.split("<text ")
				.find(|line| line.contains(&format!(">{text}<")))
				.unwrap();
			line.split("y=\"")
				.nth(1)
				.and_then(|rest| rest.split('"').next())
				.unwrap()
				.parse::<f32>()
				.unwrap()
		};
		let (credit, credited) = svg(
			DocMeta {
				authors: vec!["Pete Hayman".into()],
				created: Date::parse("2026-11-21").ok(),
				..default()
			},
			Some(CardMedia {
				png: png(1, 1, [0; 4]),
				video: false,
			}),
		);
		// the longest usual line holds one line of the narrow column
		credit.xpect_eq(vec!["PETE HAYMAN · 21 NOVEMBER 2026".to_string()]);
		let credit = baseline(&credited, "PETE HAYMAN · 21 NOVEMBER 2026");
		let described = baseline(&credited, "about it");
		(baseline(&credited, "A Post") < credit && credit < described)
			.xpect_true();
		let (credit, bare) = svg(default(), None);
		credit.xpect_empty();
		(baseline(&bare, "about it") < described).xpect_true();
		// a line too long for the column breaks before its date
		let (credit, _) = svg(
			DocMeta {
				authors: vec!["Beetmania".into(), "Beet Lover".into()],
				created: Date::parse("2026-11-21").ok(),
				..default()
			},
			Some(CardMedia {
				png: png(1, 1, [0; 4]),
				video: false,
			}),
		);
		credit.xpect_eq(vec![
			"BEETMANIA, BEET LOVER".to_string(),
			"21 NOVEMBER 2026".to_string(),
		]);
	}

	/// The description fills the room above the bottom row and no more,
	/// whatever the blocks above it took.
	#[beet_core::test]
	fn fills_the_room_left() {
		let font = CardFont::new(FONT.to_vec()).unwrap();
		let lines = |title: &str, media: bool| {
			let page = CardPage {
				path: fwd_path!("post"),
				meta: DocMeta {
					title: Some(title.into()),
					description: Some("a beet in every pot ".repeat(60)),
					created: Date::parse("2026-11-21").ok(),
					..default()
				},
				source: None,
			};
			let media = media.then(|| CardMedia {
				png: png(1, 1, [0; 4]),
				video: false,
			});
			let has_media = media.is_some();
			let card =
				Card::new(&font, &PackageConfig::default(), &page, media)
					.unwrap();
			let above = CardLayout::credit(has_media).end(
				CardLayout::title(has_media).end(
					CardLayout::top(has_media),
					0.,
					card.title.len(),
				),
				CardLayout::CREDIT_GAP,
				card.credit.len(),
			);
			let style = CardLayout::description(has_media, above);
			let end = style.end(above, CardLayout::GAP, card.description.len());
			(end <= CardLayout::FLOOR
				&& end + style.line_height > CardLayout::FLOOR)
				.xpect_true();
			card.description.len()
		};
		lines("Beets", true)
			.xpect_greater_than(lines(&"Beets ".repeat(9), true));
		lines("Beets", false).xpect_greater_than(2);
	}

	/// The alt text is the card's words, each block closed as a sentence.
	#[beet_core::test]
	fn writes_the_alt_text() {
		CardText::new(
			&DocMeta {
				title: Some("Beets!".into()),
				description: Some("about beets".into()),
				authors: vec!["Pete Hayman".into()],
				created: Date::parse("2025-07-11").ok(),
				..default()
			},
			&default(),
		)
		.alt()
		.xpect_eq("Beets! Pete Hayman, 11 July 2025. about beets.");
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
				let width = font.width(line, style).unwrap();
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
			.all(|line| font.width(line, style).unwrap() <= style.max_width)
			.xpect_true();
	}
}

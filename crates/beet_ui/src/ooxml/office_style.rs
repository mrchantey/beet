//! The style mapping every Office reader and writer shares, both directions
//! by the [module docs](super)' Style section: a reader's direct looks
//! registered as typed inline rules, a theme slot read as its token, and a
//! writer's looks resolved through the cascade's pure resolve.
use crate::prelude::*;
use crate::style::FontStyle;
use crate::style::FontWeight;
use crate::style::common_props::BackgroundColor;
use crate::style::common_props::*;
use crate::style::material::colors;
use crate::style::*;
use beet_core::prelude::*;
use beet_net::prelude::RequestParts;
use beet_scene::prelude::*;

/// How a reader reads a file's looks, held by every Office reader and set
/// through its `with_options`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Get, SetWith)]
pub struct OfficeReadOptions {
	/// Where a direct look lands in the cascade.
	direct_formatting: DirectFormatting,
	/// How a colour a file names by its theme's slot reads.
	theme_colors: ThemeColors,
}

/// Where a reader registers the looks a file formats directly.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DirectFormatting {
	/// In the [`Direct`](RuleOrigin::Direct) origin, outranking every author
	/// rule as an HTML `style` attribute does.
	#[default]
	Override,
	/// In the [`Author`](RuleOrigin::Author) origin at class specificity, so
	/// a more specific theme rule wins.
	Cascade,
	/// Not read: the theme alone styles the document.
	Drop,
}

/// How a reader reads a colour named by its theme's slot.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ThemeColors {
	/// As the token the slot maps to (see [`ThemeSlot::token`]), so the
	/// document takes the world's theme and scheme.
	#[default]
	Tokens,
	/// As the colour it computes to in the file's own theme.
	Verbatim,
}

/// A slot of an Office theme's colour scheme, in the order a writer's slot
/// table lists them, the first winning when two slots hold one colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeSlot {
	/// `dk1`, the text colour, `tx1`.
	Dark1,
	/// `lt1`, the background colour, `bg1`.
	Light1,
	/// `dk2`, `tx2`.
	Dark2,
	/// `lt2`, `bg2`.
	Light2,
	/// `accent1`.
	Accent1,
	/// `accent2`.
	Accent2,
	/// `accent3`.
	Accent3,
	/// `accent4`.
	Accent4,
	/// `accent5`.
	Accent5,
	/// `accent6`.
	Accent6,
	/// `hlink`, a hyperlink.
	Hyperlink,
	/// `folHlink`, a followed hyperlink.
	FollowedHyperlink,
}

/// A colour as a file names it: a value of its own or a slot of its theme,
/// tinted toward white (positive) or shaded toward black (negative).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OfficeColor {
	/// A colour of its own.
	Rgb(Color),
	/// A theme slot, its luminance tinted by `tint` from -1 to 1.
	Slot {
		/// The slot.
		slot: ThemeSlot,
		/// The tint, 0 for the slot's own colour.
		tint: f32,
	},
}

/// A file's theme: the colour of each slot, Office's own default theme where
/// a file has none.
#[derive(Debug, Clone, PartialEq)]
pub struct OfficeTheme {
	/// Each slot's colour, in [`ThemeSlot::ALL`] order.
	colors: [Color; 12],
}

/// What a file formats directly on one element, each field absent where
/// the file says nothing, so the cascade decides.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct DirectLook {
	/// The text colour.
	pub color: Option<OfficeColor>,
	/// The background fill.
	pub background: Option<OfficeColor>,
	/// The font size in points.
	pub font_size_pt: Option<f32>,
	/// Bold, or explicitly not.
	pub bold: Option<bool>,
	/// Italic, or explicitly not.
	pub italic: Option<bool>,
	/// Underlined, or explicitly not.
	pub underline: Option<bool>,
	/// Struck through, or explicitly not.
	pub strike: Option<bool>,
	/// The horizontal alignment.
	pub text_align: Option<TextAlign>,
}

/// The conversions between CSS's units and Office's, by CSS's own ratios:
/// 96 px and 72 pt to the inch, 16 px to the rem.
pub struct OfficeUnits;

/// The look a writer writes for one element: what Office formats can say of
/// a resolved style, each field absent where it says nothing, ie in a
/// [`diff`](Self::diff) where an element resolves as its base does.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct OfficeLook {
	/// Bold.
	pub bold: Option<bool>,
	/// Italic.
	pub italic: Option<bool>,
	/// Underlined.
	pub underline: Option<bool>,
	/// Struck through.
	pub strike: Option<bool>,
	/// The text colour.
	pub color: Option<Color>,
	/// The background fill.
	pub background: Option<Color>,
	/// The font size in px.
	pub font_size_px: Option<f32>,
	/// The horizontal alignment.
	pub text_align: Option<TextAlign>,
}

/// The looks a writer writes, resolved through the cascade's pure resolve
/// for its scheme, medium and viewport: the root's as the format's
/// defaults, each named style's as a probe of its tag resolves under the
/// root, and the theme's palette by the slot table.
#[derive(Debug, Clone)]
pub struct OfficeStyleSheet {
	/// The tree's root, every probe spawned beneath it.
	root: Entity,
	/// The media every look is resolved for.
	media: MediaContext,
	/// The root's look, the format's document defaults.
	defaults: OfficeLook,
	/// Each slot's colour for the writer's scheme.
	palette: OfficeTheme,
	/// Whether a theme's tokens resolved the palette, without which it is
	/// Office's default and no colour names a slot.
	themed: bool,
}

/// One element of a probe chain: a bare element of its tag, and of its
/// class where the structure a reader reads names one, ie a footnote
/// definition's.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProbeTag {
	/// The tag.
	pub tag: SmolStr,
	/// The class.
	pub class: Option<SmolStr>,
}

impl ProbeTag {
	/// A probe of `tag`.
	pub fn new(tag: impl Into<SmolStr>) -> Self {
		Self {
			tag: tag.into(),
			class: None,
		}
	}

	/// A probe of `tag` carrying `class`.
	pub fn with_class(
		tag: impl Into<SmolStr>,
		class: impl Into<SmolStr>,
	) -> Self {
		Self {
			tag: tag.into(),
			class: Some(class.into()),
		}
	}
}

impl ThemeSlot {
	/// Every slot in table order.
	pub const ALL: [Self; 12] = [
		Self::Dark1,
		Self::Light1,
		Self::Dark2,
		Self::Light2,
		Self::Accent1,
		Self::Accent2,
		Self::Accent3,
		Self::Accent4,
		Self::Accent5,
		Self::Accent6,
		Self::Hyperlink,
		Self::FollowedHyperlink,
	];

	/// The slot a DrawingML scheme colour or a theme part's element names,
	/// `dk1` or its alias `tx1`, or a Word `w:themeColor` names, `dark1` or
	/// its alias `text1`.
	pub fn parse(name: &str) -> Option<Self> {
		match name {
			"dk1" | "tx1" | "dark1" | "text1" => Some(Self::Dark1),
			"lt1" | "bg1" | "light1" | "background1" => Some(Self::Light1),
			"dk2" | "tx2" | "dark2" | "text2" => Some(Self::Dark2),
			"lt2" | "bg2" | "light2" | "background2" => Some(Self::Light2),
			"accent1" => Some(Self::Accent1),
			"accent2" => Some(Self::Accent2),
			"accent3" => Some(Self::Accent3),
			"accent4" => Some(Self::Accent4),
			"accent5" => Some(Self::Accent5),
			"accent6" => Some(Self::Accent6),
			"hlink" | "hyperlink" => Some(Self::Hyperlink),
			"folHlink" | "followedHyperlink" => Some(Self::FollowedHyperlink),
			_ => None,
		}
	}

	/// The name a Word `w:themeColor` gives this slot, the inverse of
	/// [`parse`](Self::parse) for Word.
	pub fn word_name(self) -> &'static str {
		match self {
			Self::Dark1 => "text1",
			Self::Light1 => "background1",
			Self::Dark2 => "text2",
			Self::Light2 => "background2",
			Self::Hyperlink => "hyperlink",
			Self::FollowedHyperlink => "followedHyperlink",
			other => other.name(),
		}
	}

	/// The slot a workbook's `theme` index names, SpreadsheetML counting the
	/// light slot of each of the first two pairs before the dark.
	pub fn from_spreadsheet_index(index: u32) -> Option<Self> {
		match index {
			0 => Some(Self::Light1),
			1 => Some(Self::Dark1),
			2 => Some(Self::Light2),
			3 => Some(Self::Dark2),
			4 => Some(Self::Accent1),
			5 => Some(Self::Accent2),
			6 => Some(Self::Accent3),
			7 => Some(Self::Accent4),
			8 => Some(Self::Accent5),
			9 => Some(Self::Accent6),
			10 => Some(Self::Hyperlink),
			11 => Some(Self::FollowedHyperlink),
			_ => None,
		}
	}

	/// The index a workbook names this slot by, the inverse of
	/// [`from_spreadsheet_index`](Self::from_spreadsheet_index).
	pub fn spreadsheet_index(self) -> u32 {
		(0..12)
			.find(|index| Self::from_spreadsheet_index(*index) == Some(self))
			.unwrap_or(0)
	}

	/// The element naming this slot in a theme part's colour scheme.
	pub fn name(self) -> &'static str {
		match self {
			Self::Dark1 => "dk1",
			Self::Light1 => "lt1",
			Self::Dark2 => "dk2",
			Self::Light2 => "lt2",
			Self::Accent1 => "accent1",
			Self::Accent2 => "accent2",
			Self::Accent3 => "accent3",
			Self::Accent4 => "accent4",
			Self::Accent5 => "accent5",
			Self::Accent6 => "accent6",
			Self::Hyperlink => "hlink",
			Self::FollowedHyperlink => "folHlink",
		}
	}

	/// The token the slot reads as, by the module docs' slot table.
	pub fn token(self) -> Token {
		match self {
			Self::Dark1 => colors::OnSurface.into(),
			Self::Light1 => colors::Surface.into(),
			Self::Dark2 => colors::OnSurfaceVariant.into(),
			Self::Light2 => colors::SurfaceContainer.into(),
			Self::Accent1 | Self::Hyperlink => colors::Primary.into(),
			Self::Accent2 => colors::Secondary.into(),
			Self::Accent3 | Self::FollowedHyperlink => colors::Tertiary.into(),
			Self::Accent4 => colors::PrimaryContainer.into(),
			Self::Accent5 => colors::SecondaryContainer.into(),
			Self::Accent6 => colors::TertiaryContainer.into(),
		}
	}

	fn index(self) -> usize {
		Self::ALL.iter().position(|slot| *slot == self).unwrap_or(0)
	}
}

impl Default for OfficeTheme {
	/// Office's own default theme.
	fn default() -> Self {
		Self {
			colors: [
				"000000", "FFFFFF", "44546A", "E7E6E6", "4472C4", "ED7D31",
				"A5A5A5", "FFC000", "5B9BD5", "70AD47", "0563C1", "954F72",
			]
			.map(|hex| OfficeColor::parse_hex(hex).unwrap_or(Color::BLACK)),
		}
	}
}

impl OfficeTheme {
	/// The theme a theme part's `a:clrScheme` declares, a slot it omits
	/// keeping Office's default.
	pub fn parse(nodes: &[BsxNode]) -> Self {
		let mut theme = Self::default();
		let Some(scheme) = BsxNode::document_element(nodes).and_then(|root| {
			root.descendants_named("clrScheme").first().copied()
		}) else {
			return theme;
		};
		for element in scheme.elements() {
			let Some(slot) = ThemeSlot::parse(element.local_name()) else {
				continue;
			};
			let color = element.elements().find_map(|value| {
				match value.local_name() {
					"srgbClr" => value.attribute("val"),
					// a system colour carries the value it last was
					"sysClr" => value.attribute("lastClr"),
					_ => None,
				}
				.and_then(OfficeColor::parse_hex)
			});
			if let Some(color) = color {
				theme.colors[slot.index()] = color;
			}
		}
		theme
	}

	/// The colour of `slot`.
	pub fn color(&self, slot: ThemeSlot) -> Color { self.colors[slot.index()] }

	/// This theme with `slot` holding `color`.
	pub fn with_color(mut self, slot: ThemeSlot, color: Color) -> Self {
		self.colors[slot.index()] = color;
		self
	}

	/// The first slot in the table holding `color`, compared as the
	/// eight bit channels a file writes.
	pub fn slot_of(&self, color: Color) -> Option<ThemeSlot> {
		let hex = OfficeColor::hex(color);
		ThemeSlot::ALL
			.into_iter()
			.find(|slot| OfficeColor::hex(self.color(*slot)) == hex)
	}

	/// A theme part declaring this palette as its colour scheme, with a
	/// font scheme and the format scheme every theme part must carry.
	pub fn theme_part(&self) -> Vec<u8> {
		let mut xml = super::xml_part::XmlPart::new();
		xml.open("a:theme", &[
			("xmlns:a", OoxmlNamespace::DRAWING),
			("name", "beet"),
		])
		.open("a:themeElements", &[])
		.open("a:clrScheme", &[("name", "beet")]);
		for slot in ThemeSlot::ALL {
			xml.open(&format!("a:{}", slot.name()), &[])
				.empty("a:srgbClr", &[(
					"val",
					&OfficeColor::hex(self.color(slot)),
				)])
				.close(&format!("a:{}", slot.name()));
		}
		xml.close("a:clrScheme").raw(Self::FONT_AND_FORMAT_SCHEMES);
		xml.close("a:themeElements").close("a:theme");
		xml.into_bytes()
	}

	/// The font scheme and the minimal format scheme a theme part requires:
	/// three fills, lines, effects and background fills.
	const FONT_AND_FORMAT_SCHEMES: &str = "<a:fontScheme name=\"beet\">\
		<a:majorFont><a:latin typeface=\"Calibri Light\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:majorFont>\
		<a:minorFont><a:latin typeface=\"Calibri\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:minorFont>\
		</a:fontScheme><a:fmtScheme name=\"beet\"><a:fillStyleLst>\
		<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
		<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
		<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
		</a:fillStyleLst><a:lnStyleLst>\
		<a:ln w=\"6350\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln>\
		<a:ln w=\"12700\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln>\
		<a:ln w=\"19050\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln>\
		</a:lnStyleLst><a:effectStyleLst>\
		<a:effectStyle><a:effectLst/></a:effectStyle>\
		<a:effectStyle><a:effectLst/></a:effectStyle>\
		<a:effectStyle><a:effectLst/></a:effectStyle>\
		</a:effectStyleLst><a:bgFillStyleLst>\
		<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
		<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
		<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
		</a:bgFillStyleLst></a:fmtScheme>";
}

impl OfficeColor {
	/// A colour from its six hex digits, ie `FF0000`, or eight with a
	/// leading alpha, ie a workbook's `FFFF0000`.
	pub fn parse_hex(hex: &str) -> Option<Color> {
		let hex = hex.trim().trim_start_matches('#');
		let hex = match hex.len() {
			8 => &hex[2..],
			6 => hex,
			_ => return None,
		};
		Srgba::hex(hex).ok().map(Color::from)
	}

	/// The six uppercase hex digits a file writes `color` as, ie `FF0000`.
	pub fn hex(color: Color) -> String {
		let srgba = color.to_srgba();
		let channel = |value: f32| (value.clamp(0., 1.) * 255.).round() as u8;
		format!(
			"{:02X}{:02X}{:02X}",
			channel(srgba.red),
			channel(srgba.green),
			channel(srgba.blue)
		)
	}

	/// The colour this computes to in `theme`.
	pub fn compute(&self, theme: &OfficeTheme) -> Color {
		match *self {
			Self::Rgb(color) => color,
			Self::Slot { slot, tint } => Self::tinted(theme.color(slot), tint),
		}
	}

	/// What a reader declares for this colour: a slot's token under
	/// [`ThemeColors::Tokens`] when it is the slot's own colour, else the
	/// colour it computes to in `theme`.
	pub fn declaration(
		&self,
		options: &OfficeReadOptions,
		theme: &OfficeTheme,
	) -> Result<TokenValue> {
		match *self {
			Self::Slot { slot, tint }
				if tint == 0.
					&& *options.theme_colors() == ThemeColors::Tokens =>
			{
				TokenValue::token(slot.token()).xok()
			}
			_ => TypedValue::new(self.compute(theme))?
				.xinto::<TokenValue>()
				.xok(),
		}
	}

	/// `color` with its HSL luminance tinted toward white by a positive
	/// `tint` or shaded toward black by a negative one, as SpreadsheetML
	/// applies a `tint`.
	pub fn tinted(color: Color, tint: f32) -> Color {
		if tint == 0. {
			return color;
		}
		let mut hsla = Hsla::from(color);
		hsla.lightness = match tint < 0. {
			true => hsla.lightness * (1. + tint),
			false => hsla.lightness * (1. - tint) + tint,
		};
		Color::from(hsla)
	}
}

impl DirectLook {
	/// Whether the file says nothing of this element's look.
	pub fn is_empty(&self) -> bool { *self == Self::default() }

	/// Declares this look on `entity` as an [`InlineRule`] in the origin
	/// `options` names, keyed by its declarations so equal looks share one
	/// class, `Drop` and an empty look declaring nothing.
	pub fn apply(
		&self,
		entity: &mut EntityWorldMut,
		options: &OfficeReadOptions,
		theme: &OfficeTheme,
	) -> Result {
		let origin = match options.direct_formatting() {
			DirectFormatting::Override => RuleOrigin::Direct,
			DirectFormatting::Cascade => RuleOrigin::Author,
			DirectFormatting::Drop => return Ok(()),
		};
		let declarations = self.declarations(options, theme)?;
		if declarations.is_empty() {
			return Ok(());
		}
		// the class keyed by what it declares, so equal looks share it
		let mut key = format!("office-{origin:?}");
		let mut sorted = declarations
			.iter()
			.map(|(token, value)| format!("{token}={value:?}"))
			.collect::<Vec<_>>();
		sorted.sort();
		for declaration in sorted {
			key.push(';');
			key.push_str(&declaration);
		}
		let class = ClassName::from_inline_source(&key);
		entity.insert(InlineRule::new(&class, origin, declarations));
		Ok(())
	}

	/// The declarations this look makes.
	fn declarations(
		&self,
		options: &OfficeReadOptions,
		theme: &OfficeTheme,
	) -> Result<Vec<(TokenKey, TokenValue)>> {
		let mut out = Vec::new();
		let mut push = |token: Token, value: TokenValue| {
			out.push((token.key().clone(), value));
		};
		if let Some(color) = &self.color {
			push(ForegroundColor.into(), color.declaration(options, theme)?);
		}
		if let Some(background) = &self.background {
			push(
				BackgroundColor.into(),
				background.declaration(options, theme)?,
			);
		}
		if let Some(pt) = self.font_size_pt {
			push(
				FontSize.into(),
				TypedValue::new(Length::Px(OfficeUnits::px_from_pt(pt)))?
					.into(),
			);
		}
		if let Some(bold) = self.bold {
			let weight = match bold {
				true => FontWeight::Bold,
				false => FontWeight::Normal,
			};
			push(FontWeightProp.into(), TypedValue::new(weight)?.into());
		}
		if let Some(italic) = self.italic {
			let style = match italic {
				true => FontStyle::Italic,
				false => FontStyle::Normal,
			};
			push(FontStyleProp.into(), TypedValue::new(style)?.into());
		}
		if self.underline.is_some() || self.strike.is_some() {
			let line = DecorationLine {
				underline: self.underline.unwrap_or(false),
				line_through: self.strike.unwrap_or(false),
				..DecorationLine::DEFAULT
			};
			push(DecorationLineProp.into(), TypedValue::new(line)?.into());
		}
		if let Some(align) = self.text_align {
			push(TextAlignProp.into(), TypedValue::new(align)?.into());
		}
		out.xok()
	}
}

impl OfficeUnits {
	/// CSS px to the inch.
	pub const PX_PER_INCH: f32 = 96.;
	/// English Metric Units per inch.
	pub const EMU_PER_INCH: f32 = 914400.;
	/// Points to the inch.
	pub const PT_PER_INCH: f32 = 72.;
	/// CSS px to the rem.
	pub const PX_PER_REM: f32 = 16.;

	/// `px` in points, 1 px being 0.75 pt.
	pub fn pt(px: f32) -> f32 { px * Self::PT_PER_INCH / Self::PX_PER_INCH }

	/// `pt` in px.
	pub fn px_from_pt(pt: f32) -> f32 {
		pt * Self::PX_PER_INCH / Self::PT_PER_INCH
	}

	/// `px` in half points, Word's font size unit, rounded.
	pub fn half_points(px: f32) -> u32 { (Self::pt(px) * 2.).round() as u32 }

	/// `px` in twentieths of a point, Word's spacing unit, rounded: 1 px is
	/// 15 twips.
	pub fn twips(px: f32) -> i64 { (Self::pt(px) * 20.).round() as i64 }

	/// `px` in English Metric Units, DrawingML's: 1 px is 9525 EMU.
	pub fn emu(px: f32) -> i64 {
		(px * Self::EMU_PER_INCH / Self::PX_PER_INCH).round() as i64
	}

	/// `emu` English Metric Units in px.
	pub fn px_from_emu(emu: f32) -> f32 {
		emu * Self::PX_PER_INCH / Self::EMU_PER_INCH
	}

	/// `length` in px, a percentage of `of`, ie the page's content box, and
	/// a viewport length of `viewport`.
	pub fn px(length: Length, of: f32, viewport: Option<MediaViewport>) -> f32 {
		match length {
			Length::Px(px) => px,
			Length::Rem(rem) => rem * Self::PX_PER_REM,
			Length::Percent(percent) => percent / 100. * of,
			other => {
				let viewport = viewport.unwrap_or_default();
				other.into_px(bevy::math::Vec2::new(
					viewport.width_px(),
					viewport.height_px(),
				))
			}
		}
	}
}

impl OfficeLook {
	/// The look a resolved style writes as, its font size against a root
	/// of 16 px.
	pub fn from_computed(style: &ComputedStyle) -> Self {
		let visual = &style.visual;
		Self {
			bold: Some(visual.font_weight.is_bold()),
			italic: Some(visual.font_style == FontStyle::Italic),
			underline: Some(visual.decoration_line.underline),
			strike: Some(visual.decoration_line.line_through),
			color: visual.foreground,
			background: visual.background,
			font_size_px: Some(OfficeUnits::px(
				visual.font_size,
				OfficeUnits::PX_PER_REM,
				None,
			)),
			text_align: Some(visual.text_align),
		}
	}

	/// What this look says differently from `base`, a colour only where
	/// this resolves one.
	pub fn diff(&self, base: &Self) -> Self {
		fn differ<T: Clone + PartialEq>(
			look: &Option<T>,
			base: &Option<T>,
		) -> Option<T> {
			(look != base).then(|| look.clone()).flatten()
		}
		Self {
			bold: differ(&self.bold, &base.bold),
			italic: differ(&self.italic, &base.italic),
			underline: differ(&self.underline, &base.underline),
			strike: differ(&self.strike, &base.strike),
			color: differ(&self.color, &base.color),
			background: differ(&self.background, &base.background),
			font_size_px: differ(&self.font_size_px, &base.font_size_px),
			text_align: differ(&self.text_align, &base.text_align),
		}
	}

	/// Whether this says nothing.
	pub fn is_empty(&self) -> bool { *self == Self::default() }
}

impl OfficeStyleSheet {
	/// The scheme a write resolves for: the request's `color-scheme` param,
	/// else the theme's own, else the tree's own classes.
	pub fn scheme(
		world: &World,
		request: &RequestParts,
	) -> Option<ColorScheme> {
		request
			.get_param("color-scheme")
			.and_then(ColorScheme::parse)
			.or_else(|| {
				world
					.get_resource::<crate::style::material::Theme>()
					.map(|theme| theme.scheme)
			})
	}

	/// Resolves the looks of `root` for `media`: the tree's styles brought
	/// up to date by a [`PostParseTree`] run, the root's look and the theme's
	/// palette. A named style's look is a [`probe`](Self::probe) of its tag.
	pub fn resolve(
		world: &mut World,
		root: Entity,
		media: MediaContext,
	) -> Result<Self> {
		let _ = world.try_run_schedule(PostParseTree);
		let resolved =
			world.with_state::<CascadeQuery, _>(|cascade| -> Result<_> {
				let memo = &mut CascadeMemo::for_media(media);
				let defaults =
					OfficeLook::from_computed(&cascade.resolve(root, memo)?);
				let mut palette = OfficeTheme::default();
				let mut themed = false;
				for slot in ThemeSlot::ALL {
					if let Some(color) = cascade
						.rules()
						.resolve_untyped(root, &slot.token(), memo)
						.ok()
						.and_then(|value| value.into_serde::<Color>().ok())
					{
						palette = palette.with_color(slot, color);
						themed = true;
					}
				}
				(defaults, palette, themed).xok()
			});
		let (defaults, palette, themed) = resolved?;
		Self {
			root,
			media,
			defaults,
			palette,
			themed,
		}
		.xok()
	}

	/// The media every look is resolved for.
	pub fn media(&self) -> MediaContext { self.media }

	/// The slot a colour a writer writes names: the first in the table
	/// holding it, none where no theme resolved the palette.
	pub fn theme_slot(&self, color: Color) -> Option<ThemeSlot> {
		self.themed.then(|| self.palette.slot_of(color)).flatten()
	}

	/// The computed style of every element of each chain, resolved as a
	/// chain of bare elements spawned under the root and despawned once
	/// resolved, so a writer learns what the structure a reader will read
	/// back resolves to with no look of its own.
	pub fn probe(
		&self,
		world: &mut World,
		chains: &[Vec<ProbeTag>],
	) -> Result<Vec<Vec<ComputedStyle>>> {
		let spawned = chains
			.iter()
			.map(|chain| {
				let mut parent = self.root;
				chain
					.iter()
					.map(|probe| {
						let entity = world
							.spawn((
								Element::new(probe.tag.clone()),
								ChildOf(parent),
							))
							.id();
						if let Some(class) = &probe.class {
							world.spawn((
								AttributeOf::new(entity),
								Attribute::new("class"),
								Value::Str(class.clone()),
							));
						}
						parent = entity;
						entity
					})
					.collect::<Vec<_>>()
			})
			.collect::<Vec<_>>();
		let resolved = spawned
			.iter()
			.map(|chain| self.computed(world, chain))
			.collect::<Result<Vec<_>>>();
		for chain in spawned {
			if let Some(first) = chain.first() {
				world.entity_mut(*first).despawn();
			}
		}
		resolved
	}

	/// The computed styles of `entities` for the writer's media in one pass.
	pub fn computed(
		&self,
		world: &mut World,
		entities: &[Entity],
	) -> Result<Vec<ComputedStyle>> {
		world.with_state::<CascadeQuery, _>(|cascade| {
			let memo = &mut CascadeMemo::for_media(self.media);
			entities
				.iter()
				.map(|entity| cascade.resolve(*entity, memo))
				.collect()
		})
	}

	/// Whether each of `entities` resolves `break-after: page`.
	pub fn breaks_after(
		&self,
		world: &mut World,
		entities: &[Entity],
	) -> Result<Vec<bool>> {
		world.with_state::<RuleSetQuery, _>(|rules| {
			let memo = &mut CascadeMemo::for_media(self.media);
			entities
				.iter()
				.map(|entity| {
					rules
						.resolve(*entity, BreakAfterProp, memo)
						.map(|value| value == BreakAfter::Page)
						.unwrap_or(false)
				})
				.collect::<Vec<_>>()
				.xok()
		})
	}

	/// The root's look, the format's document defaults.
	pub fn defaults(&self) -> &OfficeLook { &self.defaults }

	/// The theme's palette for the writer's scheme.
	pub fn palette(&self) -> &OfficeTheme { &self.palette }

	/// The full looks of `entities`, resolved for the writer's media in one
	/// pass.
	pub fn looks(
		&self,
		world: &mut World,
		entities: &[Entity],
	) -> Result<Vec<OfficeLook>> {
		self.computed(world, entities)?
			.iter()
			.map(OfficeLook::from_computed)
			.collect::<Vec<_>>()
			.xok()
	}

	/// What each of `entities` writes as direct formatting: what it resolves
	/// differently from `base`, else from the defaults.
	pub fn direct(
		&self,
		world: &mut World,
		entities: &[Entity],
		base: Option<&OfficeLook>,
	) -> Result<Vec<OfficeLook>> {
		let base = base.unwrap_or(&self.defaults);
		self.looks(world, entities)?
			.into_iter()
			.map(|look| look.diff(base))
			.collect::<Vec<_>>()
			.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::style::common_props::*;
	use crate::style::material::*;
	use crate::style::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn converts_units() {
		// 1 px is 0.75 pt, 15 twips and 9525 EMU, a rem 16 px
		OfficeUnits::pt(16.).xpect_eq(12.);
		OfficeUnits::half_points(16.).xpect_eq(24);
		OfficeUnits::twips(1.).xpect_eq(15);
		OfficeUnits::emu(1.).xpect_eq(9525);
		let rem = OfficeUnits::px(Length::Rem(1.5), 0., None);
		rem.xpect_eq(24.);
		OfficeUnits::pt(rem).xpect_eq(18.);
		OfficeUnits::half_points(rem).xpect_eq(36);
		OfficeUnits::twips(rem).xpect_eq(360);
		OfficeUnits::emu(rem).xpect_eq(228600);
		OfficeUnits::px(Length::Percent(50.), 602., None).xpect_eq(301.);
		OfficeUnits::px_from_pt(12.).xpect_eq(16.);
	}

	/// A world styled by the Material theme, holding a light themed page.
	fn themed(scene: impl Bundle) -> (World, Entity) {
		let mut world =
			(MaterialStylePlugin::default(), StylePlugin).into_world();
		let root = world.spawn((scene, Classes::new([classes::PAGE]))).id();
		(world, root)
	}

	/// The entity of the sole element `tag`.
	fn tagged(world: &mut World, tag: &str) -> Entity {
		world
			.query::<(Entity, &Element)>()
			.iter(world)
			.find(|(_, element)| element.tag() == tag)
			.map(|(entity, _)| entity)
			.unwrap()
	}

	fn screen() -> MediaContext {
		MediaContext::new(Medium::Screen).with_scheme(ColorScheme::Light)
	}

	/// A direct red run under a themed root writes red, and the root's own
	/// colour writes as the defaults, never on a run.
	#[beet_core::test]
	fn direct_runs_write_what_differs() {
		let (mut world, root) =
			themed(rsx! { <div><span>"red"</span><em>"plain"</em></div> });
		let span = tagged(&mut world, "span");
		DirectLook {
			color: Some(OfficeColor::Rgb(Color::srgb(1., 0., 0.))),
			..default()
		}
		.apply(
			&mut world.entity_mut(span),
			&OfficeReadOptions::default(),
			&OfficeTheme::default(),
		)
		.unwrap();
		let em = tagged(&mut world, "em");
		let sheet =
			OfficeStyleSheet::resolve(&mut world, root, screen()).unwrap();
		let on_surface = sheet.palette().color(ThemeSlot::Dark1);
		sheet.defaults().color.xpect_eq(Some(on_surface));
		let direct = sheet.direct(&mut world, &[span, em], None).unwrap();
		direct[0].color.xpect_eq(Some(Color::srgb(1., 0., 0.)));
		direct[1].clone().xpect_eq(OfficeLook {
			italic: Some(true),
			..default()
		});
	}

	/// An `h1` a theme rule styles writes its look into the heading style,
	/// and a run inside it writes nothing of its own.
	#[beet_core::test]
	fn named_styles_carry_their_tags_look() {
		let (mut world, root) =
			themed(rsx! { <div><h1><span>"Title"</span></h1></div> });
		let span = tagged(&mut world, "span");
		let sheet =
			OfficeStyleSheet::resolve(&mut world, root, screen()).unwrap();
		let heading = sheet
			.probe(&mut world, &[vec![ProbeTag::new("h1")]])
			.unwrap()
			.remove(0)
			.remove(0)
			.xmap(|style| OfficeLook::from_computed(&style));
		heading.bold.xpect_eq(Some(true));
		(heading.font_size_px > sheet.defaults().font_size_px).xpect_true();
		sheet
			.direct(&mut world, &[span], Some(&heading))
			.unwrap()
			.remove(0)
			.is_empty()
			.xpect_true();
	}

	/// A `Print` rule shapes a print write, a `Terminal` rule never does.
	#[beet_core::test]
	fn print_rules_shape_a_print_write() {
		let (mut world, root) = themed(rsx! { <div><p>"words"</p></div> });
		world.resource_mut::<RuleSet>().extend_rules([
			Rule::tag("p")
				.with_media(MediaQuery::Print)
				.with_canonical(FontWeight::Bold),
			Rule::tag("p")
				.with_media(MediaQuery::Terminal)
				.with_canonical(FontStyle::Italic),
		]);
		let para = tagged(&mut world, "p");
		let print = MediaContext::new(Medium::Print);
		let sheet = OfficeStyleSheet::resolve(&mut world, root, print).unwrap();
		let look = sheet.looks(&mut world, &[para]).unwrap().remove(0);
		(look.bold, look.italic).xpect_eq((Some(true), Some(false)));
	}

	/// The options each read: `accent1` as `Primary` under `Tokens` and its
	/// hex under `Verbatim`, nothing under `Drop`, and under `Cascade` a more
	/// specific author rule wins.
	#[beet_core::test]
	fn read_options() {
		let accent = DirectLook {
			color: Some(OfficeColor::Slot {
				slot: ThemeSlot::Accent1,
				tint: 0.,
			}),
			..default()
		};
		let theme = OfficeTheme::default();
		let foreground = |options: OfficeReadOptions, look: &DirectLook| {
			let (mut world, root) =
				themed(rsx! { <div><p class="note">"x"</p></div> });
			world.resource_mut::<RuleSet>().insert_rule(
				Rule::new()
					.with_selector(Selector::AllOf(vec![
						Selector::tag("p"),
						Selector::class("note"),
					]))
					.with_value(ForegroundColor, Color::srgb(0., 0., 1.)),
			);
			let para = tagged(&mut world, "p");
			look.apply(&mut world.entity_mut(para), &options, &theme)
				.unwrap();
			world.with_state::<RuleSetQuery, _>(|query| {
				let memo = &mut CascadeMemo::for_media(screen());
				(
					query.resolve(para, ForegroundColor, memo).ok(),
					query.resolve(root, colors::Primary, memo).ok(),
				)
			})
		};
		let (tokens, primary) =
			foreground(OfficeReadOptions::default(), &accent);
		tokens.xpect_eq(primary);
		foreground(
			OfficeReadOptions::default()
				.with_theme_colors(ThemeColors::Verbatim),
			&accent,
		)
		.0
		.map(OfficeColor::hex)
		.xpect_eq(Some("4472C4".to_string()));
		let red = DirectLook {
			color: Some(OfficeColor::Rgb(Color::srgb(1., 0., 0.))),
			..default()
		};
		let blue = Some(Color::srgb(0., 0., 1.));
		foreground(
			OfficeReadOptions::default()
				.with_direct_formatting(DirectFormatting::Drop),
			&red,
		)
		.0
		.xpect_eq(blue);
		foreground(
			OfficeReadOptions::default()
				.with_direct_formatting(DirectFormatting::Cascade),
			&red,
		)
		.0
		.xpect_eq(blue);
		foreground(OfficeReadOptions::default(), &red)
			.0
			.xpect_eq(Some(Color::srgb(1., 0., 0.)));
	}

	/// A theme part's colour scheme reads back as written, and a slot's
	/// tint lightens it.
	#[beet_core::test]
	fn reads_and_writes_a_theme_part() {
		let theme = OfficeTheme::default()
			.with_color(ThemeSlot::Accent1, Color::srgb(1., 0., 0.));
		let read = OfficeTheme::parse(
			&OpcPackage::read_xml(&theme.theme_part()).unwrap(),
		);
		read.clone().xpect_eq(theme);
		read.slot_of(Color::srgb(1., 0., 0.))
			.xpect_eq(Some(ThemeSlot::Accent1));
		OfficeColor::hex(OfficeColor::tinted(Color::BLACK, 0.5))
			.xpect_eq("808080");
	}
}

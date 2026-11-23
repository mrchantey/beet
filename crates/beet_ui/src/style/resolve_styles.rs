use crate::prelude::*;
use crate::style::AlignSelf;
use crate::style::FlexBox;
use crate::style::LayoutStyle;
use crate::style::VisualStyle;
use crate::style::common_props::BackgroundColor;
use crate::style::common_props::*;
use crate::style::*;
use beet_core::prelude::*;

/// An entity's computed style, the cascade's target agnostic output: what
/// every renderer reads, the charcell painter from the components a live
/// surface keeps and a document writer from a pure resolve.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ComputedStyle {
	/// Colours, text decoration, font and alignment.
	pub visual: VisualStyle,
	/// The box's display and flex or grid layout.
	pub layout: LayoutStyle,
	/// Margins, borders, padding and sizes; default for a non-element.
	pub box_style: BoxStyle,
	/// Positioning; default for a non-element.
	pub position: PositionStyle,
	/// Scrollbar colours and width; default for a non-element.
	pub scrollbar: ScrollbarStyle,
	/// Transition settings.
	pub transition: TransitionStyle,
}

/// The pure resolve: an entity's [`ComputedStyle`] from the tree, the
/// [`RuleSet`] and the media a [`CascadeMemo`] states, reading the world and
/// writing nothing, so a writer may resolve a tree for its file while the
/// same tree is on screen. [`resolve_styles`] is its caching driver for live
/// surfaces.
#[derive(SystemParam)]
pub struct CascadeQuery<'w, 's> {
	rules: RuleSetQuery<'w, 's>,
	// the box model (margin/border/padding/background) is element-level; text
	// and fragment nodes must not resolve their nearest ancestor's box and
	// re-paint it.
	elements: Query<'w, 's, (), With<Element>>,
	// a tag-less text node, the anonymous box a run of text lays out as
	texts: Query<'w, 's, (), (With<Value>, Without<Element>)>,
}

impl CascadeQuery<'_, '_> {
	/// The cascade this resolves through.
	pub fn rules(&self) -> &RuleSetQuery<'_, '_> { &self.rules }

	/// `entity`'s computed style for the media `memo` states, else the media
	/// of the surface it renders under.
	pub fn resolve(
		&self,
		entity: Entity,
		memo: &mut CascadeMemo,
	) -> Result<ComputedStyle> {
		let rules = &self.rules;
		let is_element = self.elements.contains(entity);
		// the box this node is, see `LayoutBox`
		let layout_box = match (is_element, self.texts.contains(entity)) {
			(true, _) => LayoutBox::Element,
			(false, true) => LayoutBox::Text,
			(false, false) => LayoutBox::Fragment,
		};
		ComputedStyle {
			visual: resolve_visual(rules, entity, memo)?,
			layout: resolve_layout(rules, entity, layout_box, memo)?,
			// element-level, so a text or fragment child does not inherit and
			// re-paint its ancestor element's border, background or position
			box_style: match is_element {
				true => resolve_box(rules, entity, memo)?,
				false => default(),
			},
			position: match is_element {
				true => resolve_position(rules, entity, memo)?,
				false => default(),
			},
			scrollbar: match is_element {
				true => resolve_scrollbar(rules, entity, memo)?,
				false => default(),
			},
			// every node, not just elements: a text leaf resolves through its
			// nearest element's rules, so it carries the same transition and
			// its inherited colours ease in step with the element
			transition: resolve_transition(rules, entity, memo)?,
		}
		.xok()
	}
}

/// Resolves every changed tree's [`ComputedStyle`] into the components a
/// live surface keeps, through [`CascadeQuery`], each written only when it
/// changed.
pub(crate) fn resolve_styles(
	mut commands: Commands,
	cascade: CascadeQuery,
	// classes drive the cascade (eg `.dark-scheme`), so a runtime class change
	// must re-resolve even when the [`Element`] itself is untouched. Interactive
	// state (eg `:focus`) does too, so a focus change re-resolves the cascade.
	query: Query<
		Entity,
		Or<(Changed<Element>, Changed<Classes>, Changed<ElementStateMap>)>,
	>,
	// a resized surface re-evaluates width-gated media rules for its whole
	// tree. `MediaViewport` is `set_if_neq`-maintained (`sync_media_viewport`),
	// so this fires on a real resize or first sight, not on paint's per-frame
	// buffer writes.
	resized: Query<Entity, Changed<MediaViewport>>,
	// an attribute selector (`[hidden]`) reads an element's attributes, so an
	// attribute added after the element first resolved re-resolves it, ie a
	// card deck hiding the cards a render does not show
	attributes: Query<
		&AttributeOf,
		Or<(Changed<AttributeOf>, Changed<Attribute>)>,
	>,
	ancestors: Query<&ChildOf>,
	// content transcluded by reference has no `ChildOf` edge to the layout, so the
	// traversal follows holders to re-resolve referenced content under the layout's
	// cascade (eg the color scheme), even when the content itself is unchanged.
	tree: RenderTreeQuery,
	// the nodes the cascade owns: what the change filters above watch
	owned: Query<(), Or<(With<Element>, With<Classes>, With<ElementStateMap>)>>,
	mut styles: Query<(
		Option<&mut VisualStyle>,
		Option<&mut LayoutStyle>,
		Option<&mut BoxStyle>,
		Option<&mut PositionStyle>,
		Option<&mut ScrollbarStyle>,
	)>,
	mut transitions: Query<Option<&mut TransitionStyle>>,
) -> Result {
	let mut roots = query
		.iter()
		.chain(attributes.iter().map(|attribute| attribute.get()))
		.map(|entity| ancestors.root_ancestor(entity))
		.collect::<HashSet<_>>();
	// a resized surface seeds the traversal (its whole tree, portals included,
	// re-resolves below) — only worth it when a width-gated rule exists to
	// change the outcome. The seeds are the nearest nodes the cascade owns:
	// a bare buffer host, or a text node carrying an authored style, is walked
	// through rather than resolved, so a resize never touches a node nothing
	// in the cascade would otherwise reach.
	if !resized.is_empty() && cascade.rules().has_width_media() {
		let mut pending = resized
			.iter()
			.map(|entity| tree.resolve(entity))
			.collect::<Vec<_>>();
		while let Some(entity) = pending.pop() {
			if owned.contains(entity) {
				roots.insert(entity);
				continue;
			}
			pending.extend(tree.children(entity));
		}
	}

	// within-pass cascade memo, keyed by `(entity, token)`. Resolving the page
	// touches ~30 properties per entity and inherited tokens re-walk ancestors;
	// without this the ancestor re-walk is O(n²). Fresh each run so no stale
	// value leaks across frames.
	let mut memo = CascadeMemo::default();

	// inheritance cache friendly parallelism, top down queue,
	// as described in stylo https://youtu.be/Y6SSTRr2mFU?t=310
	// a root walked up from a node under a held prefab is the held entity
	// itself, which Bevy's default filters leave out of every style query,
	// as they do an authored `Disabled` one, and nothing renders
	let mut queue = roots
		.into_iter()
		.map(|root| tree.resolve(root))
		.filter(|root| styles.contains(*root))
		.collect::<Vec<_>>();
	while !queue.is_empty() {
		for entity in queue.drain(..).collect::<Vec<_>>() {
			let computed = cascade.resolve(entity, &mut memo)?;
			let (visual, layout, box_style, position, scrollbar) =
				styles.get_mut(entity)?;
			match visual {
				Some(mut style) => {
					style.set_if_neq(computed.visual);
				}
				None => {
					commands.entity(entity).insert(computed.visual);
				}
			}
			match layout {
				Some(mut style) => {
					style.set_if_neq(computed.layout);
				}
				None => {
					commands.entity(entity).insert(computed.layout);
				}
			}
			match box_style {
				Some(mut style) => {
					style.set_if_neq(computed.box_style);
				}
				None => {
					commands.entity(entity).insert(computed.box_style);
				}
			}
			match position {
				Some(mut style) => {
					style.set_if_neq(computed.position);
				}
				None => {
					commands.entity(entity).insert(computed.position);
				}
			}
			match scrollbar {
				Some(mut style) => {
					style.set_if_neq(computed.scrollbar);
				}
				None => {
					commands.entity(entity).insert(computed.scrollbar);
				}
			}
			// attached only while the duration is nonzero, so the animation
			// system iterates transitioned nodes alone
			let transition = computed.transition;
			match transitions.get_mut(entity)? {
				Some(mut style) if transition.duration > Duration::ZERO => {
					style.set_if_neq(transition);
				}
				Some(_) => {
					commands
						.entity(entity)
						.remove::<(TransitionStyle, VisualTransition)>();
				}
				None if transition.duration > Duration::ZERO => {
					commands.entity(entity).insert(transition);
				}
				None => {}
			}

			// a `Portal` holder is replaced by the content it renders in place,
			// so transcluded content re-resolves under this (layout) cascade. An
			// unresolved holder (no `Portal` yet) has no content to cascade into.
			queue.extend(tree.children(entity));
		}
	}
	Ok(())
}

fn resolve_visual(
	query: &RuleSetQuery,
	entity: Entity,
	memo: &mut CascadeMemo,
) -> Result<VisualStyle> {
	let foreground = query.resolve(entity, ForegroundColor, memo).ok();
	let background = query.resolve(entity, BackgroundColor, memo).ok();
	let decoration_color = query.resolve(entity, DecorationColor, memo).ok();
	let decoration_line = query
		.resolve(entity, DecorationLineProp, memo)
		.unwrap_or_default();
	let decoration_style = query
		.resolve(entity, DecorationStyleProp, memo)
		.unwrap_or_default();
	let text_align = query
		.resolve(entity, TextAlignProp, memo)
		.unwrap_or_default();
	let font_weight = query
		.resolve(entity, FontWeightProp, memo)
		.unwrap_or_default();
	let font_style = query
		.resolve(entity, FontStyleProp, memo)
		.unwrap_or_default();
	// font-size drives charcell glyph scaling. MD3 sets it per-heading (`<h1>`..)
	// and on the `text-*` typography classes, so the standard cascade (own beats
	// inherited, class beats tag) resolves the effective size; absent, text is
	// 1em (normal single-cell glyphs).
	let font_size = query
		.resolve(entity, FontSize, memo)
		.unwrap_or(Length::Rem(1.0));
	let blink = query
		.resolve(entity, BlinkStyleProp, memo)
		.unwrap_or_default();
	let visibility = query
		.resolve(entity, VisibilityProp, memo)
		.unwrap_or_default();
	// `opacity` bakes into the resolved colours (the charcell approximation,
	// see [`VisualStyle::apply_opacity`]), so a transition eases the dim.
	let opacity = query.resolve(entity, OpacityProp, memo).unwrap_or(1.);

	let mut style = VisualStyle {
		foreground,
		background,
		decoration_color,
		decoration_line,
		decoration_style,
		font_weight,
		font_style,
		font_size,
		blink,
		visibility,
		text_align,
	};
	style.apply_opacity(opacity);
	style.xok()
}

/// The box a node's resolved [`LayoutStyle`] describes. A non-element matches
/// its nearest element's rules, yet it is an anonymous box rather than that
/// element, so only some of what it matches is its own.
#[derive(Debug, Clone, Copy, PartialEq)]
enum LayoutBox {
	/// An element, every prop its own.
	Element,
	/// A tag-less text run: in a flex container an anonymous item (CSS
	/// `flex: 0 1 auto`), so it reads only its element's `display` and the
	/// inherited `white-space`, never the element's own sizing or overflow.
	Text,
	/// A tag-less node kept in the flow (eg a slot carrying its element's
	/// background): it lays its children out as its element would, so it reads
	/// the element's container props, but the element's own `flex-*`, `order`
	/// and `align-self` place the element in its parent's line, never a box
	/// inside it, so those stay initial.
	Fragment,
}

/// Resolve `entity`'s [`LayoutStyle`] as the `layout_box` it is.
fn resolve_layout(
	query: &RuleSetQuery,
	entity: Entity,
	layout_box: LayoutBox,
	memo: &mut CascadeMemo,
) -> Result<LayoutStyle> {
	let display = query.resolve(entity, DisplayProp, memo).unwrap_or_default();
	let white_space = query
		.resolve(entity, WhiteSpaceProp, memo)
		.unwrap_or_default();
	if layout_box == LayoutBox::Text {
		return LayoutStyle {
			display,
			white_space,
			..default()
		}
		.xok();
	}
	let flex_grow = query
		.resolve(entity, FlexGrowProp, memo)
		.unwrap_or(LayoutStyle::DEFAULT.flex_grow);
	let flex_shrink = query
		.resolve(entity, FlexShrinkProp, memo)
		.unwrap_or(LayoutStyle::DEFAULT.flex_shrink);
	let flex_basis = query.resolve(entity, FlexBasisProp, memo).ok();
	let flex_order = query
		.resolve(entity, FlexOrderProp, memo)
		.unwrap_or_default();
	let align_self = query
		.resolve(entity, AlignSelfProp, memo)
		.unwrap_or_default();
	let direction = query
		.resolve(entity, FlexDirectionProp, memo)
		.unwrap_or_default();
	let wrap = query
		.resolve(entity, FlexWrapProp, memo)
		.unwrap_or_default();
	let justify_content = query
		.resolve(entity, JustifyContentProp, memo)
		.unwrap_or_default();
	let align_items = query
		.resolve(entity, AlignItemsProp, memo)
		.unwrap_or_default();
	let align_content = query
		.resolve(entity, AlignContentProp, memo)
		.unwrap_or_default();
	// gaps stay as `Length` here (the resolution-independent value): each renderer
	// converts at layout time, where the real viewport is known. The charcell
	// engine rounds to whole cells via `FlexBox::{row,column}_gap_cells`.
	let row_gap = query.resolve(entity, RowGapProp, memo).unwrap_or_default();
	let column_gap = query
		.resolve(entity, ColumnGapProp, memo)
		.unwrap_or_default();
	let overflow_x = query
		.resolve(entity, OverflowXProp, memo)
		.unwrap_or_default();
	let overflow_y = query
		.resolve(entity, OverflowYProp, memo)
		.unwrap_or_default();
	let grid = GridTracks {
		columns: query
			.resolve(entity, GridTemplateColumnsProp, memo)
			.unwrap_or_default(),
		rows: query
			.resolve(entity, GridAutoRowsProp, memo)
			.unwrap_or_default(),
	};
	LayoutStyle {
		display,
		white_space,
		flex_box: FlexBox {
			direction,
			wrap,
			justify_content,
			align_items,
			align_content,
			row_gap,
			column_gap,
		},
		grid,
		flex_grow,
		flex_shrink,
		flex_basis,
		flex_order,
		align_self,
		overflow_x,
		overflow_y,
	}
	.xmap(|layout| match layout_box {
		LayoutBox::Fragment => LayoutStyle {
			flex_grow: LayoutStyle::DEFAULT.flex_grow,
			flex_shrink: LayoutStyle::DEFAULT.flex_shrink,
			flex_basis: None,
			flex_order: LayoutStyle::DEFAULT.flex_order,
			align_self: AlignSelf::Auto,
			..layout
		},
		_ => layout,
	})
	.xok()
}

fn resolve_position(
	query: &RuleSetQuery,
	entity: Entity,
	memo: &mut CascadeMemo,
) -> Result<PositionStyle> {
	let position = query
		.resolve(entity, PositionProp, memo)
		.unwrap_or_default();
	// each inset is `auto` (None) unless a rule sets it.
	PositionStyle {
		position,
		inset: [
			query.resolve(entity, InsetTop, memo).ok(),
			query.resolve(entity, InsetRight, memo).ok(),
			query.resolve(entity, InsetBottom, memo).ok(),
			query.resolve(entity, InsetLeft, memo).ok(),
		],
		z_index: query.resolve(entity, ZIndexProp, memo).ok(),
		try_fallbacks: query
			.resolve(entity, PositionTryProp, memo)
			.unwrap_or_default(),
	}
	.xok()
}

fn resolve_scrollbar(
	query: &RuleSetQuery,
	entity: Entity,
	memo: &mut CascadeMemo,
) -> Result<ScrollbarStyle> {
	let width = query
		.resolve(entity, ScrollbarWidthProp, memo)
		.unwrap_or_default();
	// scrollbar-color sets both thumb and track; absent leaves renderer defaults.
	let color = query.resolve(entity, ScrollbarColorProp, memo).ok();
	ScrollbarStyle {
		thumb: color.map(|c| c.thumb),
		track: color.map(|c| c.track),
		width,
	}
	.xok()
}

fn resolve_transition(
	query: &RuleSetQuery,
	entity: Entity,
	memo: &mut CascadeMemo,
) -> Result<TransitionStyle> {
	let default = TransitionStyle::default();
	TransitionStyle {
		duration: query
			.resolve(entity, TransitionDurationProp, memo)
			.unwrap_or(default.duration),
		ease: query
			.resolve(entity, TransitionEaseProp, memo)
			.unwrap_or(default.ease),
	}
	.xok()
}

fn resolve_box(
	query: &RuleSetQuery,
	entity: Entity,
	memo: &mut CascadeMemo,
) -> Result<BoxStyle> {
	let padding = query.resolve(entity, Padding, memo).unwrap_or_default();
	let margin = query.resolve(entity, MarginProp, memo).unwrap_or_default();
	let border_color = query.resolve(entity, BorderColorProp, memo).ok();
	// per-side border widths fall back to the uniform `border-width`, so a rule
	// can reserve a single edge (eg `border-right`) or a full box.
	let uniform = query.resolve(entity, OutlineWidth, memo).ok();
	let resolve_side = |width: Result<Length>| {
		width.ok().or(uniform).unwrap_or(Length::DEFAULT)
	};
	BoxStyle {
		border_left: border_color,
		border_right: border_color,
		border_top: border_color,
		border_bottom: border_color,
		border: Spacing {
			top: resolve_side(query.resolve(entity, BorderTopWidth, memo)),
			right: resolve_side(query.resolve(entity, BorderRightWidth, memo)),
			bottom: resolve_side(query.resolve(
				entity,
				BorderBottomWidth,
				memo,
			)),
			left: resolve_side(query.resolve(entity, BorderLeftWidth, memo)),
		},
		margin,
		padding,
		width: query.resolve(entity, Width, memo).ok(),
		min_width: query.resolve(entity, MinWidth, memo).ok(),
		max_width: query.resolve(entity, MaxWidth, memo).ok(),
		height: query.resolve(entity, Height, memo).ok(),
		min_height: query.resolve(entity, MinHeight, memo).ok(),
		max_height: query.resolve(entity, MaxHeight, memo).ok(),
	}
	.xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::style::material::MaterialStylePlugin;
	use crate::style::*;
	use beet_core::prelude::*;

	/// The page of a tree in `scheme`, resolved live.
	fn page(world: &mut World, scheme: ColorScheme) -> Entity {
		world
			.spawn((rsx! { <div/> }, Classes::new([classes::PAGE]), scheme))
			.id()
	}

	/// A pure resolve stating the light scheme over a dark tree answers the
	/// light tree's tones, and leaves the dark tree's components as they were.
	#[beet_core::test]
	fn pure_resolve_states_its_scheme() {
		let mut world =
			(MaterialStylePlugin::default(), StylePlugin).into_world();
		let dark = page(&mut world, ColorScheme::Dark);
		let light = page(&mut world, ColorScheme::Light);
		world.run_schedule(PostParseTree);
		let live = |world: &World, entity: Entity| {
			world.entity(entity).get::<VisualStyle>().unwrap().clone()
		};
		let (dark_live, light_live) = (live(&world, dark), live(&world, light));
		(dark_live.foreground != light_live.foreground).xpect_true();
		world
			.with_state::<CascadeQuery, _>(|cascade| {
				cascade
					.resolve(
						dark,
						&mut CascadeMemo::for_media(
							MediaContext::new(Medium::Screen)
								.with_scheme(ColorScheme::Light),
						),
					)
					.unwrap()
					.visual
					.foreground
			})
			.xpect_eq(light_live.foreground);
		live(&world, dark).xpect_eq(dark_live);
	}

	/// An attribute added after its element first resolved re-resolves it,
	/// so an attribute selector applies, ie `[hidden]` in the terminal.
	#[beet_core::test]
	fn resolves_an_added_attribute() {
		let mut world = StylePlugin.into_world();
		world.spawn(rsx! { <div><p>"words"</p></div> });
		let paragraph = world
			.query::<(Entity, &Element)>()
			.iter(&world)
			.find(|(_, element)| element.tag() == "p")
			.unwrap()
			.0;
		world.run_schedule(PostParseTree);
		let display = |world: &World| {
			world
				.entity(paragraph)
				.get::<LayoutStyle>()
				.unwrap()
				.display
		};
		(display(&world) == Display::None).xpect_false();
		world.spawn((
			AttributeOf::new(paragraph),
			Attribute::new("hidden"),
			Value::Null,
		));
		world.run_schedule(PostParseTree);
		(display(&world) == Display::None).xpect_true();
	}
}

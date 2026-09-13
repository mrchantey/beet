//! The scene editor's classes and their rules, colocated with the widgets that
//! emit them and registered into the global [`RuleSet`] at build.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::*;
use crate::style::material::*;
use crate::style::Display;

// ── Class names ─────────────────────────────────────────────────────────────────

/// The `<details>` a `ToggleSceneEditor` opens the editor from.
pub(crate) const SCENE_EDITOR_TOGGLE: ClassName = ClassName::new_static("scene-editor-toggle");
/// The editor's frame: the tree beside the inspector.
pub(crate) const SCENE_EDITOR: ClassName = ClassName::new_static("scene-editor");
/// The tree's column.
pub(crate) const SCENE_EDITOR_TREE: ClassName = ClassName::new_static("scene-editor-tree");
/// The inspector's column.
pub(crate) const SCENE_EDITOR_INSPECTOR: ClassName = ClassName::new_static("scene-editor-inspector");
/// The tree itself, the rows' holder.
pub(crate) const SCENE_TREE: ClassName = ClassName::new_static("scene-tree");
/// One row of the tree, selectable.
pub(crate) const SCENE_TREE_ROW: ClassName = ClassName::new_static("scene-tree-row");
/// The guide glyphs drawing a row's place in the tree.
pub(crate) const SCENE_TREE_GUIDES: ClassName = ClassName::new_static("scene-tree-guides");
/// The inspector itself, its generation's holder.
pub(crate) const SCENE_INSPECTOR: ClassName = ClassName::new_static("scene-inspector");
/// The inspector's header: the entity's title and the structural edits.
pub(crate) const SCENE_INSPECTOR_HEADER: ClassName = ClassName::new_static("scene-inspector-header");
/// The selected entity's title in the inspector's header.
pub(crate) const SCENE_INSPECTOR_TITLE: ClassName = ClassName::new_static("scene-inspector-title");
/// The header's row of structural edits.
pub(crate) const SCENE_INSPECTOR_ACTIONS: ClassName = ClassName::new_static("scene-inspector-actions");

/// Viewport width (px) at or below which the tree stacks above the inspector
/// rather than sitting beside it, the sidebar's breakpoint so the two agree.
const STACK_BREAKPOINT_PX: u32 = classes::SIDEBAR_BREAKPOINT_PX;

// ── Rules ─────────────────────────────────────────────────────────────────────

/// Every rule the scene editor's classes need, in cascade order.
pub(super) fn scene_editor_rules() -> Vec<Rule> {
	vec![
		scene_editor(),
		scene_editor_stacked(),
		scene_editor_tree(),
		scene_editor_tree_terminal(),
		scene_editor_inspector(),
		scene_tree_row(),
		scene_tree_row_active(),
		scene_tree_row_selected(),
		scene_tree_guides(),
		scene_inspector_header(),
		scene_inspector_actions(),
		scene_inspector_title(),
	]
}

/// The editor frame: a row of the two columns, each stretched to the frame's
/// height, with a gap between them.
fn scene_editor() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SCENE_EDITOR))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexDirectionProp, Direction::Horizontal)
		.with_value(common_props::AlignItemsProp, AlignItems::Stretch)
		.with_value(common_props::ColumnGapProp, Length::Rem(1.))
		.with_value(common_props::RowGapProp, Length::Rem(1.))
}

/// Below the breakpoint the columns stack: a narrow terminal or phone has no
/// room for a tree beside a form.
fn scene_editor_stacked() -> Rule {
	Rule::new()
		.with_media(MediaQuery::MaxWidth(STACK_BREAKPOINT_PX))
		.with_selector(Selector::class(SCENE_EDITOR))
		.with_value(common_props::FlexDirectionProp, Direction::Vertical)
}

/// The tree's column: a fixed rail on the web, on a low surface so the
/// selection reads against it. Inset horizontally only, so its first row
/// lines up with the inspector's title beside it.
fn scene_editor_tree() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SCENE_EDITOR_TREE))
		.with_value(common_props::Width, Length::Rem(20.))
		.with_value(common_props::MinWidth, Length::Rem(14.))
		.with_token(common_props::BackgroundColor, colors::SurfaceContainerLow).unwrap()
		.with_token(ShapeProps, geometry::ShapeSmall).unwrap()
		.with_value(common_props::Padding, Spacing {
			left: Length::Rem(0.5),
			right: Length::Rem(0.5),
			..Spacing::DEFAULT
		})
}

/// The terminal's tree rail, in cells: wide enough for three guide columns
/// and a label before wrapping.
fn scene_editor_tree_terminal() -> Rule {
	Rule::new()
		.with_media(MediaQuery::Terminal)
		.with_selector(Selector::class(SCENE_EDITOR_TREE))
		.with_value(common_props::Width, Length::Rem(34.))
		.with_value(common_props::MinWidth, Length::Rem(24.))
}

/// The inspector's column takes the rest of the frame.
fn scene_editor_inspector() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SCENE_EDITOR_INSPECTOR))
		.with_value(common_props::FlexGrowProp, 1u32)
		.with_value(common_props::MinWidth, Length::Px(0.))
}

/// A tree row: a full-width block so the whole row is the click target, the
/// muted ink of a navigation item, a pointer to say it is one.
fn scene_tree_row() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SCENE_TREE_ROW))
		.with_value(common_props::DisplayProp, Display::Block)
		.with_value(common_props::CursorProp, Cursor::Pointer)
		.with_token(common_props::ForegroundColor, colors::OnSurfaceVariant).unwrap()
		.with_token(ShapeProps, geometry::ShapeExtraSmall).unwrap()
		.with_value(common_props::Padding, Spacing {
			left: Length::Rem(0.5),
			right: Length::Rem(0.5),
			..Spacing::DEFAULT
		})
}

/// A hovered or focused row, lifted so the keyboard's place in the tree is
/// visible.
fn scene_tree_row_active() -> Rule {
	let active = |state: ElementState| {
		Selector::AllOf(vec![
			Selector::class(SCENE_TREE_ROW),
			Selector::state(state),
		])
	};
	Rule::new()
		.with_selector(Selector::AnyOf(vec![
			active(ElementState::Focused),
			active(ElementState::Hovered),
		]))
		.with_token(common_props::ForegroundColor, colors::OnSurface).unwrap()
		.with_token(common_props::BackgroundColor, colors::SurfaceContainerHigh).unwrap()
}

/// The selected row: primary ink on a raised surface, the same treatment the
/// sidebar gives the current page. An attribute selector, so it works the
/// same on both targets.
fn scene_tree_row_selected() -> Rule {
	Rule::new()
		.with_selector(Selector::AllOf(vec![
			Selector::class(SCENE_TREE_ROW),
			Selector::attribute("aria-selected", Some("true".into())),
		]))
		.with_token(common_props::ForegroundColor, colors::Primary).unwrap()
		.with_token(common_props::BackgroundColor, colors::SurfaceContainerHighest).unwrap()
		.with_token(common_props::FontWeightProp, typography::WeightMedium).unwrap()
}

/// The guide glyphs: faint, so the labels read first.
fn scene_tree_guides() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SCENE_TREE_GUIDES))
		.with_token(common_props::ForegroundColor, colors::Outline).unwrap()
}

/// The inspector's header: the title over the row of structural edits, with
/// the form a gap below.
fn scene_inspector_header() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SCENE_INSPECTOR_HEADER))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexDirectionProp, Direction::Vertical)
		.with_value(common_props::AlignItemsProp, AlignItems::Start)
		.with_value(common_props::RowGapProp, Length::Rem(0.5))
		.with_value(common_props::MarginProp, Spacing {
			bottom: Length::Rem(1.),
			..Spacing::DEFAULT
		})
}

/// The header's buttons, side by side.
fn scene_inspector_actions() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SCENE_INSPECTOR_ACTIONS))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexDirectionProp, Direction::Horizontal)
		.with_value(common_props::AlignItemsProp, AlignItems::Center)
		.with_value(common_props::ColumnGapProp, Length::Px(0.))
}

/// The selected entity's title, at the weight of a group heading.
fn scene_inspector_title() -> Rule {
	Rule::new()
		.with_selector(Selector::class(SCENE_INSPECTOR_TITLE))
		.with_token(TypographyProps, typography::TitleMedium).unwrap()
}

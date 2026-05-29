//! The shared class-name vocabulary, as [`ClassName`] constants.
//!
//! Class names are the contract between the widgets that *emit* them and the
//! rule set that *styles* them. The vocabulary lives here in `token` — the
//! always-available home of [`ClassName`]/[`Classes`](crate::token::Classes) —
//! so both layers reference one source of truth: widgets ([`scene`] feature)
//! emit these classes and a rule set ([`style`] feature, Material Design 3
//! today) maps them to design tokens. Neither layer owns the names, and the
//! `style` layer stays usable for raw markdown without pulling in widgets.
//!
//! [`scene`]: https://docs.rs/beet_ui
//! [`style`]: https://docs.rs/beet_ui
#![cfg_attr(rustfmt, rustfmt_skip)]
use super::ClassName;

// ── Color scheme ──────────────────────────────────────────────────────────────
// Applied to an ancestor element (eg the document root); styled by the
// `light_scheme`/`dark_scheme` rules and themed down the cascade.
pub const LIGHT_SCHEME: ClassName = ClassName::new_static("light-scheme");
pub const DARK_SCHEME: ClassName = ClassName::new_static("dark-scheme");

// ── Buttons ───────────────────────────────────────────────────────────────────
pub const BTN: ClassName = ClassName::new_static("btn");
pub const BTN_FILLED: ClassName = ClassName::new_static("btn-filled");
pub const BTN_OUTLINED: ClassName = ClassName::new_static("btn-outlined");
pub const BTN_TEXT: ClassName = ClassName::new_static("btn-text");
pub const BTN_TONAL: ClassName = ClassName::new_static("btn-tonal");
pub const BTN_ELEVATED: ClassName = ClassName::new_static("btn-elevated");
pub const BTN_SECONDARY: ClassName = ClassName::new_static("btn-secondary");
pub const BTN_TERTIARY: ClassName = ClassName::new_static("btn-tertiary");
pub const BTN_ERROR: ClassName = ClassName::new_static("btn-error");
pub const BTN_ICON: ClassName = ClassName::new_static("btn-icon");

// ── Cards ─────────────────────────────────────────────────────────────────────
pub const CARD_FILLED: ClassName = ClassName::new_static("card-filled");
pub const CARD_ELEVATED: ClassName = ClassName::new_static("card-elevated");
pub const CARD_OUTLINED: ClassName = ClassName::new_static("card-outlined");

// ── Typography scale ────────────────────────────────────────────────────────────
pub const TEXT_DISPLAY_LARGE: ClassName = ClassName::new_static("text-display-large");
pub const TEXT_DISPLAY_MEDIUM: ClassName = ClassName::new_static("text-display-medium");
pub const TEXT_DISPLAY_SMALL: ClassName = ClassName::new_static("text-display-small");
pub const TEXT_HEADLINE_LARGE: ClassName = ClassName::new_static("text-headline-large");
pub const TEXT_HEADLINE_MEDIUM: ClassName = ClassName::new_static("text-headline-medium");
pub const TEXT_HEADLINE_SMALL: ClassName = ClassName::new_static("text-headline-small");
pub const TEXT_TITLE_LARGE: ClassName = ClassName::new_static("text-title-large");
pub const TEXT_TITLE_MEDIUM: ClassName = ClassName::new_static("text-title-medium");
pub const TEXT_TITLE_SMALL: ClassName = ClassName::new_static("text-title-small");
pub const TEXT_BODY_LARGE: ClassName = ClassName::new_static("text-body-large");
pub const TEXT_BODY_MEDIUM: ClassName = ClassName::new_static("text-body-medium");
pub const TEXT_BODY_SMALL: ClassName = ClassName::new_static("text-body-small");
pub const TEXT_LABEL_LARGE: ClassName = ClassName::new_static("text-label-large");
pub const TEXT_LABEL_MEDIUM: ClassName = ClassName::new_static("text-label-medium");
pub const TEXT_LABEL_SMALL: ClassName = ClassName::new_static("text-label-small");

// ── Color / shape / elevation utilities ─────────────────────────────────────────
pub const COLOR_PRIMARY: ClassName = ClassName::new_static("color-primary");
pub const SHAPE_NONE: ClassName = ClassName::new_static("shape-none");
pub const SHAPE_EXTRA_SMALL: ClassName = ClassName::new_static("shape-xs");
pub const SHAPE_SMALL: ClassName = ClassName::new_static("shape-sm");
pub const SHAPE_MEDIUM: ClassName = ClassName::new_static("shape-md");
pub const SHAPE_LARGE: ClassName = ClassName::new_static("shape-lg");
pub const SHAPE_EXTRA_LARGE: ClassName = ClassName::new_static("shape-xl");
pub const SHAPE_FULL: ClassName = ClassName::new_static("shape-full");
pub const ELEVATION_0: ClassName = ClassName::new_static("elevation-0");
pub const ELEVATION_1: ClassName = ClassName::new_static("elevation-1");
pub const ELEVATION_2: ClassName = ClassName::new_static("elevation-2");
pub const ELEVATION_3: ClassName = ClassName::new_static("elevation-3");
pub const ELEVATION_4: ClassName = ClassName::new_static("elevation-4");
pub const ELEVATION_5: ClassName = ClassName::new_static("elevation-5");

// ── Document shell ──────────────────────────────────────────────────────────────
pub const APP_BAR: ClassName = ClassName::new_static("app-bar");
pub const APP_BAR_SCROLLED: ClassName = ClassName::new_static("app-bar-scrolled");
pub const CONTAINER: ClassName = ClassName::new_static("container");
pub const PAGE: ClassName = ClassName::new_static("page");

// ── Form controls ───────────────────────────────────────────────────────────────
pub const INPUT: ClassName = ClassName::new_static("input");
pub const INPUT_OUTLINED: ClassName = ClassName::new_static("input-outlined");
pub const INPUT_FILLED: ClassName = ClassName::new_static("input-filled");
pub const INPUT_TEXT: ClassName = ClassName::new_static("input-text");
pub const SELECT: ClassName = ClassName::new_static("select");
pub const SELECT_OUTLINED: ClassName = ClassName::new_static("select-outlined");
pub const SELECT_FILLED: ClassName = ClassName::new_static("select-filled");
pub const SELECT_TEXT: ClassName = ClassName::new_static("select-text");
pub const ERROR_TEXT: ClassName = ClassName::new_static("error-text");

// ── Table ───────────────────────────────────────────────────────────────────────
pub const TABLE: ClassName = ClassName::new_static("table");

// ── Sidebar ─────────────────────────────────────────────────────────────────────
pub const SIDEBAR: ClassName = ClassName::new_static("sidebar");
pub const SIDEBAR_LINK: ClassName = ClassName::new_static("sidebar-link");
pub const SIDEBAR_LABEL: ClassName = ClassName::new_static("sidebar-label");
pub const SIDEBAR_GROUP: ClassName = ClassName::new_static("sidebar-group");

// ── Generic utilities ─────────────────────────────────────────────────────────────
pub const HIDDEN: ClassName = ClassName::new_static("hidden");
// Print utilities, styled by `@media print` rules.
pub const PRINT_HIDDEN: ClassName = ClassName::new_static("print-hidden");
pub const PAGE_BREAK: ClassName = ClassName::new_static("page-break");
pub const TEXT_LEFT: ClassName = ClassName::new_static("text-left");
pub const TEXT_CENTER: ClassName = ClassName::new_static("text-center");
pub const TEXT_RIGHT: ClassName = ClassName::new_static("text-right");
pub const TEXT_XS: ClassName = ClassName::new_static("text-xs");
pub const TEXT_SM: ClassName = ClassName::new_static("text-sm");
pub const TEXT_BASE: ClassName = ClassName::new_static("text-base");
pub const TEXT_LG: ClassName = ClassName::new_static("text-lg");
pub const TEXT_XL: ClassName = ClassName::new_static("text-xl");
pub const TEXT_2XL: ClassName = ClassName::new_static("text-2xl");

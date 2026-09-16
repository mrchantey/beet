//! Mermaid diagrams: a ```` ```mermaid ```` fence renders as a diagram on every
//! sink, styled by the material token cascade.
//!
//! ## Modes
//!
//! The form a diagram takes is the `diagram-render` cascade property
//! ([`DiagramRender`]): `Auto`, `Svg` or `Text`. Inherited, so it is set per app
//! (a `<Rule>`), per page (`bx:style="diagram-render=Text"` on the page root) or
//! per diagram (the fence info word, ```` ```mermaid text ````).
//!
//! - `Text` is unicode box-drawing art through `mermaid-text` on every sink;
//!   flowcharts are full quality, the other types name-only boxes.
//! - `Svg` is the picture through `mermaid-rs-renderer` (the `mermaid_svg`
//!   feature, native only), inline on the web. A build without it warns and
//!   shows the text form: a lean binary keeps the diagram, only its form
//!   degrades.
//! - `Auto` is the picture on the web and text on a terminal.
//!
//! ## Passes
//!
//! Both run in [`DiagramSet`], ahead of the highlighter and the cascade:
//!
//! - [`collect_mermaid_blocks`] turns `<pre><code class="mermaid">` into a
//!   `<figure class="diagram">` carrying [`MermaidDiagram`], declaring an info
//!   word (`text | svg | auto`) as the inline rule `bx:style` would.
//! - [`materialize_diagrams`] resolves the figure's mode and builds its form
//!   beneath it: an inline `<svg>` sized by its `viewBox` and pinned to its
//!   natural width, or a `<pre class="diagram-text">` reflowed to the
//!   terminal's column budget, unbounded on the web. The sink is the surface
//!   viewport above the figure (a terminal buffer carries one, the web none),
//!   recorded with the width and mode so a tree built for the web and later
//!   painted for a terminal (a served page under the one-shot ansi renderer),
//!   or a resized terminal, rebuilds its form. A render error keeps the source
//!   visible under a material error box, and warns.
//!
//! ## Theme
//!
//! The svg is painted by role (`DiagramPaint`), each a `--diagram-*` custom
//! property the cascade declares for the figure: the `:root` defaults
//! ([`diagram_paint_defaults`]) put node fills and text on
//! `PrimaryContainer`/`OnPrimaryContainer`, borders on `Outline`, edges on
//! `OnSurfaceVariant`, labels on `OnSurface`, clusters on `SurfaceContainer`,
//! the pie and git ramp over the accent containers and fixed tones, and the
//! plain typeface; a page or a rule re-paints with
//! `bx:style="diagram-node-fill=@token:TertiaryContainer"`. On the web the
//! svg carries `var(--diagram-node-fill)` (a custom property survives the
//! stylesheet builder's variable renaming, a token variable does not), so the
//! picture follows the `.light-scheme`/`.dark-scheme` class with no
//! re-render; the font size, corner radius and stroke width are numbers the
//! crate lays out with, resolved through the cascade (`theme.rs`).
//!
//! ## Styling
//!
//! `.diagram` shares the `pre` surface (`SurfaceContainerHighest`, padding,
//! `ShapeSmall`), scrolling horizontally on the web; `.diagram > svg` fills
//! the figure's width; `.diagram-text` is the mono typeface with
//! `white-space: pre`. All are registered by
//! [`StylePlugin`](crate::style::StylePlugin) from [`diagram_rules`].
mod collect;
mod diagram;
mod materialize;
mod style;
#[cfg(all(feature = "mermaid_svg", not(target_arch = "wasm32")))]
mod svg;
#[cfg(all(feature = "mermaid_svg", not(target_arch = "wasm32")))]
mod theme;
pub(crate) use collect::*;
pub use diagram::*;
pub(crate) use materialize::*;
pub use style::*;

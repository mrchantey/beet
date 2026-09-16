//! Mermaid diagrams: a ```` ```mermaid ```` fence renders as a diagram on every
//! sink, styled by the material token cascade.
//!
//! ## Authoring
//!
//! - A fence in markdown, its info word the mode: ```` ```mermaid text ````.
//! - The `<Mermaid>` widget (`widgets/mermaid.rs`, `net` + `mermaid`) in BSX
//!   or markdown: the default slot's text as the source
//!   (`<Mermaid render="Text">graph LR; A --> B</Mermaid>`) or a file read
//!   through the nearest `BlobStore` (`<Mermaid src="docs/pipeline.mmd"/>`,
//!   the path relative to the store, ie the routes dir). It builds the same
//!   figure a fence does and materialises inline when its source lands, so it
//!   renders where no post-parse pass runs (a BSX page on the string sink).
//!   A brace in BSX text opens an expression, so a `{Decision}` node needs
//!   `src` or a fence.
//! - Per page or app: `bx:style="diagram-render=Text"` on the page root, or a
//!   `<Rule>`; the paint roles the same way (`diagram-node-fill=@token:..`).
//!
//! ## Modes
//!
//! The form a diagram takes is the `diagram-render` cascade property
//! ([`DiagramRender`]): `Auto`, `Svg` or `Text`. Inherited, so it is set per app
//! (a `<Rule>`), per page (`bx:style="diagram-render=Text"` on the page root) or
//! per diagram (the fence info word, ```` ```mermaid text ````).
//!
//! - `Text` is unicode box-drawing art through `mermaid-text` on every sink,
//!   every family it lays out (flowchart, sequence, class, state, ER, pie,
//!   gantt, git, ..) reflowed on a terminal to the columns its figure is
//!   laid out at.
//! - `Svg` is the picture through `mermaid-rs-renderer` (the `mermaid_svg`
//!   feature, native only): inline on the web, a kitty raster on a graphics
//!   terminal (the `tui` feature), text on a terminal without graphics. A
//!   build without the renderer warns and shows the text form: a lean binary
//!   keeps the diagram, only its form degrades.
//! - `Auto` is the picture on the web; on a terminal a flowchart is text (it
//!   reflows to the columns and selects), every other type a raster where the
//!   session's terminal has kitty graphics, else text.
//!
//! ## Passes
//!
//! Both run in [`DiagramSet`], ahead of the highlighter and the cascade:
//!
//! - [`collect_mermaid_blocks`] turns `<pre><code class="mermaid">` into a
//!   `<figure class="diagram">` carrying [`MermaidDiagram`], declaring an info
//!   word (`text | svg | auto`) as the inline rule `bx:style` would.
//! - [`materialize_diagrams`] resolves the figure's mode and builds its form:
//!   an inline `<svg>` beneath it, sized by its `viewBox` and pinned to its
//!   natural width; a `<pre class="diagram-text">` of art, built unbounded;
//!   or an `<img>` beneath it carrying a `KittyImage`, rasterised on the
//!   blocking pool through the same attach an `<img src>` uses, so the
//!   picture is a replaced box at the raster's natural cell size, contained
//!   by the figure (a `width: 100%` figure never stretches a small diagram).
//!   The sink is the surface viewport above the figure (a terminal buffer
//!   carries one, the web none), its entity carrying the terminal's
//!   `KittyGraphicsSupport`. The form built is recorded (`DiagramForm`) so a
//!   tree built for the web and later painted for a terminal (a served page
//!   under the one-shot ansi renderer, which has no graphics and gets text)
//!   or a session gaining or losing graphics rebuilds it, and a reparse of
//!   the fence (the collector drops the form) builds the new source. A
//!   render error keeps the source visible under a material error box, and
//!   warns; a raster failure is the `<img>`'s alt marker beside the error box.
//! - The charcell pipeline fits the art after layout
//!   (`render/charcell/reflow.rs`): nothing before layout knows the columns
//!   the figure ends up with (the docs column, a sidebar rail), so once the
//!   rects are known each `<pre>`'s art is re-rendered in place to its
//!   assigned width and the rects settle again beneath the new rows. A
//!   resize refits the same entities; the web stays unbounded and scrolls.
//! - Rendered svg is cached by source and resolved theme (`DiagramSvgCache`,
//!   `svg.rs`), so a page built again (a live reload, the next request) lays
//!   out only the diagrams whose source or paint changed.
//!
//! ## Theme
//!
//! The svg is painted by role (`DiagramPaint`), each a `--diagram-*` custom
//! property the cascade declares for the figure, the `:root` defaults
//! ([`diagram_paint_defaults`]) being the material table:
//!
//! | Role (`--diagram-*`) | Paints | Token |
//! |---|---|---|
//! | `surface` | background, edge label mask | the `pre` fill, `SurfaceContainerHighest` |
//! | `node-fill`, `node-text` | nodes, actors | `PrimaryContainer`, `OnPrimaryContainer` |
//! | `outline` | node, actor and note borders | `Outline` |
//! | `line` | edges, arrowheads | `OnSurfaceVariant` |
//! | `text` | titles, edge labels, legends | `OnSurface` |
//! | `secondary-fill` | activations | `SecondaryContainer` |
//! | `tertiary-fill`, `tertiary-text` | notes, tags | `TertiaryContainer`, `OnTertiaryContainer` |
//! | `cluster-fill`, `cluster-outline` | subgraphs, lifelines | `SurfaceContainer`, `OutlineVariant` |
//! | `ramp` | pie slices, git branches | the accent containers, fixed tones, accents |
//! | `font` | every label | `TypefacePlain` |
//!
//! Font sizes, the corner radius (`ShapeCornerSmall`) and stroke width
//! (`OutlineWidthThin`) are numbers the crate lays out with, resolved for the
//! figure in px (`theme.rs`). A page or a rule re-paints with
//! `bx:style="diagram-node-fill=@token:TertiaryContainer"`. On the web the
//! svg carries `var(--diagram-node-fill)` (a custom property survives the
//! stylesheet builder's variable renaming, a token variable does not), so the
//! picture follows the `.light-scheme`/`.dark-scheme` class with no
//! re-render; a terminal raster resolves the same roles for the figure to
//! `#rrggbb`, so a dark page rasterises in its dark tones.
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
#[cfg(all(feature = "mermaid_svg", not(target_arch = "wasm32")))]
pub(crate) use svg::DiagramSvgCache;

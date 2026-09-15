//! Mermaid diagrams: a ```` ```mermaid ```` fence renders as a diagram on every
//! sink, styled by the material token cascade.
//!
//! ## Modes
//!
//! The form a diagram takes is the `diagram-render` cascade property
//! ([`DiagramRender`]): `Auto`, `Svg` or `Text`. Inherited, so it is set per app
//! (a `<Rule>`), per page (`bx:style="diagram-render=Text"` on the page root) or
//! per diagram (the fence info word, ```` ```mermaid text ````). This build
//! renders every mode as unicode box-drawing text through `mermaid-text`;
//! flowcharts are full quality, the other types name-only boxes.
//!
//! ## Passes
//!
//! Both run in [`DiagramSet`], ahead of the highlighter and the cascade:
//!
//! - [`collect_mermaid_blocks`] turns `<pre><code class="mermaid">` into a
//!   `<figure class="diagram">` carrying [`MermaidDiagram`], declaring an info
//!   word (`text | svg | auto`) as the inline rule `bx:style` would.
//! - [`materialize_diagrams`] resolves the figure's mode and builds its form
//!   beneath it: a `<pre class="diagram-text">` reflowed to the terminal's column
//!   budget, unbounded on the web. A render error keeps the source visible under
//!   a material error box, and warns.
//!
//! ## Styling
//!
//! `.diagram` shares the `pre` surface (`SurfaceContainerHighest`, padding,
//! `ShapeSmall`), scrolling horizontally on the web; `.diagram-text` is the mono
//! typeface with `white-space: pre`. Both are registered by
//! [`StylePlugin`](crate::style::StylePlugin) from [`diagram_rules`].
mod collect;
mod diagram;
mod materialize;
mod style;
pub(crate) use collect::*;
pub use diagram::*;
pub(crate) use materialize::*;
pub use style::*;

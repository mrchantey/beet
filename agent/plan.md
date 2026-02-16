lets keep iterating on beet_stack rendering!

## `content_macro` -> `markdown!`

The purpose of content_macro was to make writing easier. actually its still pretty hard. lets give up and instead devise a markdown macro system.


```rust

pub trait IntoBundle{

}

```



## Inline Style Propagation
> **Inherited inline style propagation**: when `Important` (or similar) is a container entity (has inline markers but no `TextNode`), its markers are merged into the inherited style passed to descendant `TextNode` children. This supports both the flat `(Important, TextNode)` pattern and the container `(Important, children![TextNode])` pattern from the markdown parser.

Undo this! A TextNode and Important should be mutually exclusive, just like HTML! This will make a lot of our tests more verbose, use `markdown!`


## `InlineStyle` -> `VisitContext`

Lets formalize and share the visit context between the renderers. Renderers should only be tracking state relative to their rendering logic, not the state of the visitor itself.
I'm kinda hoping this refactor will reduce our need to need to track `heading_level` on tui_renderer to do last minute styling, like if we have some kind of style stack?
Consider how we can avoid these error prone edge cases with good ol' computer science DSAs.

```rust

struct VisitContext{
	// or does this need to be a style_stack too?
	inline_style:InlineStyle,
	in_code_block: bool,
	// or does this need to be a stack of list stacks?
	list_stack: Vec<ListCtx>,
	..
}

```


Also, for inline style consider switching to bitflags. i think that will simplify merging etc.

move all this to a new module: `src/nodes/style.rs`.

```rust
//! ratatui modifiers
use bitflags::bitflags;
pub use color::{Color, ParseColorError};
use stylize::ColorDebugKind;
pub use stylize::{Styled, Stylize};

#[cfg(feature = "anstyle")]
mod anstyle;
mod color;
pub mod palette;
#[cfg(feature = "palette")]
mod palette_conversion;
#[macro_use]
mod stylize;

bitflags! {
    /// Modifier changes the way a piece of text is displayed.
    ///
    /// They are bitflags so they can easily be composed.
    ///
    /// `From<Modifier> for Style` is implemented so you can use `Modifier` anywhere that accepts
    /// `Into<Style>`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use ratatui_core::style::Modifier;
    ///
    /// let m = Modifier::BOLD | Modifier::ITALIC;
    /// ```
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Default, Clone, Copy, Eq, PartialEq, Hash)]
    pub struct Modifier: u16 {
        const BOLD              = 0b0000_0000_0001;
        const DIM               = 0b0000_0000_0010;
        const ITALIC            = 0b0000_0000_0100;
        const UNDERLINED        = 0b0000_0000_1000;
        const SLOW_BLINK        = 0b0000_0001_0000;
        const RAPID_BLINK       = 0b0000_0010_0000;
        const REVERSED          = 0b0000_0100_0000;
        const HIDDEN            = 0b0000_1000_0000;
        const CROSSED_OUT       = 0b0001_0000_0000;
    }
}

/// Implement the `Debug` trait for `Modifier` manually.
///
/// This will avoid printing the empty modifier as 'Borders(0x0)' and instead print it as 'NONE'.
impl fmt::Debug for Modifier {
    /// Format the modifier as `NONE` if the modifier is empty or as a list of flags separated by
    /// `|` otherwise.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "NONE");
        }
        write!(f, "{}", self.0)
    }
}
```


## Testing

For more advanced 'kitchen sink' rendering and parsing tests, use `my_str.xpect_snapshot()`
Also verify that links and buttons are rendering correctly, see `hyperlink` and `button` widget for the tui stuff.

## `card_walker.rs`

- `CardWalker::entity_query`: EntityRef blocks any mutable access to any components from others. refactor this into many individual queries: `text_nodes: Query<&TextNode>, etc..`
- using `NodeKind` only in CardWalker feels like an antipattern, we have Node which stores the TypeId of the node, formalize NodeKinde and replace Node::type with Node::kind instead.

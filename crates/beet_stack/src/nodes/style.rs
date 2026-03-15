//! Inline text styling and shared visitor context.
//!
//! [`InlineStyle`] is a compact bitflag representation of inline
//! formatting markers ([`Important`](super::Important),
//! [`Emphasize`](super::Emphasize), etc.).
//!
//! [`VisitContext`] holds traversal state shared between renderers,
//! including the current entity, inline style stack, code block
//! state, list nesting, and heading level. The
//! [`CardWalker`](crate::renderers::CardWalker) maintains this
//! context and passes it to
//! [`CardVisitor`](crate::renderers::CardVisitor) methods so
//! renderers only track their own rendering-specific state.
//!
//! Merging two styles is a simple bitwise OR, which makes style
//! stack operations efficient.
use beet_core::prelude::*;
use bitflags::bitflags;
use std::fmt;

bitflags! {
	/// Inline text formatting flags.
	///
	/// Each flag corresponds to an inline marker component:
	///
	/// | Flag            | Component                            |
	/// |-----------------|--------------------------------------|
	/// | `BOLD`          | [`Important`](super::Important)      |
	/// | `ITALIC`        | [`Emphasize`](super::Emphasize)      |
	/// | `CODE`          | [`Code`](super::Code)                |
	/// | `QUOTE`         | [`Quote`](super::Quote)              |
	/// | `STRIKETHROUGH` | [`Strikethrough`](super::Strikethrough) |
	/// | `SUPERSCRIPT`   | [`Superscript`](super::Superscript)  |
	/// | `SUBSCRIPT`     | [`Subscript`](super::Subscript)      |
	/// | `MATH_INLINE`   | [`MathInline`](super::MathInline)    |
	/// | `LINK`          | [`Link`](super::Link)                |
	#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
	pub struct InlineStyle: u16 {
		/// Strong importance, ie HTML `<strong>`.
		const BOLD          = 0b0000_0000_0001;
		/// Stress emphasis, ie HTML `<em>`.
		const ITALIC        = 0b0000_0000_0010;
		/// Inline code fragment, ie HTML `<code>`.
		const CODE          = 0b0000_0000_0100;
		/// Inline quotation, ie HTML `<q>`.
		const QUOTE         = 0b0000_0000_1000;
		/// Struck-through text, ie HTML `<del>`.
		const STRIKETHROUGH = 0b0000_0001_0000;
		/// Superscript text, ie HTML `<sup>`.
		const SUPERSCRIPT   = 0b0000_0010_0000;
		/// Subscript text, ie HTML `<sub>`.
		const SUBSCRIPT     = 0b0000_0100_0000;
		/// Inline math, ie `$...$`.
		const MATH_INLINE   = 0b0000_1000_0000;
		/// Hyperlink, ie HTML `<a>`.
		const LINK          = 0b0001_0000_0000;
	}
}

impl fmt::Debug for InlineStyle {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if self.is_empty() {
			return write!(f, "NONE");
		}
		bitflags::parser::to_writer(self, f)
	}
}

impl InlineStyle {
	/// No formatting applied.
	pub fn plain() -> Self { Self::empty() }

	/// Returns true if no inline formatting is applied.
	pub fn is_plain(&self) -> bool { self.is_empty() }

	/// Merge two styles by combining flags with bitwise OR.
	///
	/// Used to inherit inline markers from ancestor containers
	/// (eg an [`Important`](super::Important) parent entity) onto
	/// descendant [`TextNode`](super::TextNode) entities via the
	/// style stack.
	pub fn merge(&self, other: &Self) -> Self { *self | *other }
}


// ---------------------------------------------------------------------------
// List context
// ---------------------------------------------------------------------------

/// Tracks the state of a single list level during traversal.
///
/// Shared between renderers via [`VisitContext`] so they don't each
/// need to maintain their own list stack.
#[derive(Debug, Clone)]
pub struct ListCtx {
	/// Whether this is an ordered (numbered) list.
	pub ordered: bool,
	/// Starting number for ordered list.
	pub start: u64,
	/// Current item index within the list (0-based).
	pub current_index: u64,
}

impl ListCtx {
	/// The display number for the current item in an ordered list.
	pub fn current_number(&self) -> u64 { self.start + self.current_index }
}


// ---------------------------------------------------------------------------
// Visit context
// ---------------------------------------------------------------------------

/// Shared traversal state passed to [`CardVisitor`](crate::renderers::CardVisitor) methods.
///
/// Maintained by [`CardWalker`](crate::renderers::CardWalker) during
/// depth-first traversal. Renderers read this context instead of
/// tracking visitor-level state themselves.
///
/// # Entity
///
/// The current entity being visited is available via
/// [`entity()`](Self::entity). The walker updates this before each
/// visitor call.
///
/// # Style Stack
///
/// When the walker enters an inline container (eg [`Important`](super::Important)
/// without a [`TextNode`](super::TextNode)), it pushes that container's
/// style onto the stack. [`effective_style`](Self::effective_style)
/// merges all stack entries to produce the current inline formatting.
#[derive(Debug, Clone)]
pub struct VisitContext {
	/// The entity currently being visited.
	entity: Entity,
	/// Stack of inline styles from ancestor containers.
	style_stack: Vec<InlineStyle>,
	/// Whether the walker is inside a [`CodeBlock`](super::CodeBlock).
	pub in_code_block: bool,
	/// Nested list context stack.
	list_stack: Vec<ListCtx>,
	/// Current heading level, or 0 if not inside a heading.
	heading_level: u8,
}

impl VisitContext {
	pub fn new(entity: Entity) -> Self {
		Self {
			entity,
			style_stack: Vec::new(),
			in_code_block: false,
			list_stack: Vec::new(),
			heading_level: 0,
		}
	}

	/// The entity currently being visited.
	pub fn entity(&self) -> Entity { self.entity }

	/// Set the current entity. Called by the walker before each
	/// visitor dispatch.
	pub fn set_entity(&mut self, entity: Entity) { self.entity = entity; }

	/// Push an inline style onto the stack when entering a container.
	pub fn push_style(&mut self, style: InlineStyle) {
		self.style_stack.push(style);
	}

	/// Pop the top inline style when leaving a container.
	pub fn pop_style(&mut self) { self.style_stack.pop(); }

	/// Compute the effective inline style by merging all stack entries.
	pub fn effective_style(&self) -> InlineStyle {
		let mut result = InlineStyle::plain();
		for entry in &self.style_stack {
			result = result.merge(entry);
		}
		result
	}

	/// Push a new list context when entering a list.
	pub fn push_list(&mut self, ordered: bool, start: u64) {
		self.list_stack.push(ListCtx {
			ordered,
			start,
			current_index: 0,
		});
	}

	/// Pop the current list context when leaving a list.
	pub fn pop_list(&mut self) { self.list_stack.pop(); }

	/// The current (innermost) list context, if inside a list.
	pub fn current_list(&self) -> Option<&ListCtx> { self.list_stack.last() }

	/// Mutable access to the current list context, eg to increment
	/// the item index.
	pub fn current_list_mut(&mut self) -> Option<&mut ListCtx> {
		self.list_stack.last_mut()
	}

	/// Current list nesting depth.
	pub fn list_depth(&self) -> usize { self.list_stack.len() }

	/// Set the heading level when entering a heading.
	pub fn set_heading_level(&mut self, level: u8) {
		self.heading_level = level;
	}

	/// Clear the heading level when leaving a heading.
	pub fn clear_heading_level(&mut self) { self.heading_level = 0; }

	/// Current heading level, or 0 if not inside a heading.
	pub fn heading_level(&self) -> u8 { self.heading_level }
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn plain_style() {
		let style = InlineStyle::plain();
		style.is_plain().xpect_true();
		style.is_empty().xpect_true();
	}

	#[test]
	fn single_flag() {
		let style = InlineStyle::BOLD;
		style.is_plain().xpect_false();
		style.contains(InlineStyle::BOLD).xpect_true();
		style.contains(InlineStyle::ITALIC).xpect_false();
	}

	#[test]
	fn combined_flags() {
		let style = InlineStyle::BOLD | InlineStyle::ITALIC;
		style.contains(InlineStyle::BOLD).xpect_true();
		style.contains(InlineStyle::ITALIC).xpect_true();
		style.contains(InlineStyle::CODE).xpect_false();
	}

	#[test]
	fn link_flag() {
		let style = InlineStyle::LINK;
		style.is_plain().xpect_false();
		style.contains(InlineStyle::LINK).xpect_true();
		style.contains(InlineStyle::BOLD).xpect_false();
	}

	#[test]
	fn merge_combines_flags() {
		let base = InlineStyle::BOLD | InlineStyle::CODE;
		let overlay = InlineStyle::ITALIC;
		let merged = base.merge(&overlay);
		merged.contains(InlineStyle::BOLD).xpect_true();
		merged.contains(InlineStyle::ITALIC).xpect_true();
		merged.contains(InlineStyle::CODE).xpect_true();
	}

	#[test]
	fn merge_combines_link() {
		let base = InlineStyle::BOLD;
		let overlay = InlineStyle::LINK;
		let merged = base.merge(&overlay);
		merged.contains(InlineStyle::BOLD).xpect_true();
		merged.contains(InlineStyle::LINK).xpect_true();
	}

	#[test]
	fn debug_empty() {
		let style = InlineStyle::empty();
		format!("{style:?}").xpect_eq("NONE");
	}

	#[test]
	fn debug_flags() {
		let style = InlineStyle::BOLD | InlineStyle::ITALIC;
		let dbg = format!("{style:?}");
		dbg.as_str().xpect_contains("BOLD");
		dbg.as_str().xpect_contains("ITALIC");
	}

	#[test]
	fn debug_link() {
		let style = InlineStyle::LINK;
		let dbg = format!("{style:?}");
		dbg.as_str().xpect_contains("LINK");
	}

	#[test]
	fn visit_context_style_stack() {
		let mut cx = VisitContext::new(Entity::PLACEHOLDER);
		cx.effective_style().is_plain().xpect_true();

		cx.push_style(InlineStyle::BOLD);
		cx.effective_style()
			.contains(InlineStyle::BOLD)
			.xpect_true();

		cx.push_style(InlineStyle::ITALIC);
		let eff = cx.effective_style();
		eff.contains(InlineStyle::BOLD).xpect_true();
		eff.contains(InlineStyle::ITALIC).xpect_true();

		cx.pop_style();
		cx.effective_style()
			.contains(InlineStyle::ITALIC)
			.xpect_false();
		cx.effective_style()
			.contains(InlineStyle::BOLD)
			.xpect_true();

		cx.pop_style();
		cx.effective_style().is_plain().xpect_true();
	}

	#[test]
	fn visit_context_list_stack() {
		let mut cx = VisitContext::new(Entity::PLACEHOLDER);
		cx.current_list().xpect_none();
		cx.list_depth().xpect_eq(0);

		cx.push_list(false, 1);
		cx.list_depth().xpect_eq(1);
		cx.current_list().unwrap().ordered.xpect_false();

		cx.push_list(true, 5);
		cx.list_depth().xpect_eq(2);
		cx.current_list().unwrap().ordered.xpect_true();
		cx.current_list().unwrap().current_number().xpect_eq(5);

		cx.current_list_mut().unwrap().current_index += 1;
		cx.current_list().unwrap().current_number().xpect_eq(6);

		cx.pop_list();
		cx.list_depth().xpect_eq(1);
		cx.current_list().unwrap().ordered.xpect_false();
	}

	#[test]
	fn visit_context_heading_level() {
		let mut cx = VisitContext::new(Entity::PLACEHOLDER);
		cx.heading_level().xpect_eq(0);

		cx.set_heading_level(2);
		cx.heading_level().xpect_eq(2);

		cx.clear_heading_level();
		cx.heading_level().xpect_eq(0);
	}
}

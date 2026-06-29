//! Inline formatting context: flow a container's inline descendants as a
//! continuous run of styled text.
//!
//! A block-level container whose children are all inline-level (text [`Value`]
//! nodes or `display: inline` elements) establishes an *inline formatting
//! context* (IFC). Rather than each descendant painting itself, the container
//! collects them into ordered `(text, style)` runs, flows the runs into lines
//! — wrapping at the content width and breaking on `\n` — and paints each
//! run's characters with that run's style. This is what lets a paragraph mix
//! plain text, emphasis, links and inline code on the same wrapped line.
use super::*;
use crate::style::Display;
use crate::style::TextAlign;
use crate::style::VisualStyle;
use crate::style::WhiteSpace;
use beet_core::prelude::*;
use bevy::math::IRect;
use bevy::math::IVec2;
use bevy::math::UVec2;

/// A contiguous run of text sharing one resolved [`VisualStyle`], sourced from
/// a single descendant node.
struct InlineRun {
	text: String,
	style: VisualStyle,
	entity: Entity,
	/// OSC-8 hyperlink inherited from the nearest `<a>`/`<img>` ancestor.
	link: Option<SmolStr>,
}

/// A styled segment of a single flowed line, ready to paint.
struct InlineSpan {
	text: String,
	style: VisualStyle,
	entity: Entity,
	link: Option<SmolStr>,
}

/// Whether `node` establishes an inline formatting context: a non-flex,
/// non-grid container all of whose children are
/// [inline-level](CharcellNodeData::is_inline_level) (flex/grid items are
/// blockified, never flowed as text).
pub(super) fn establishes_inline_flow(
	node: &CharcellNodeData,
	query: &CharcellQuery,
) -> bool {
	if matches!(node.layout_style().display, Display::Flex | Display::Grid) {
		return false;
	}
	let mut any = false;
	for child in node.child_nodes(query) {
		any = true;
		if !child.is_inline_level() {
			return false;
		}
	}
	any
}

/// Whether the container preserves whitespace and newlines (`white-space: pre`).
pub(super) fn is_preformatted(node: &CharcellNodeData) -> bool {
	node.layout_style().white_space == WhiteSpace::Pre
}

/// Cell width reserved on the left for a block list item's [`Marker`].
///
/// Non-zero only for a marker-bearing block container — an `<li>` that holds a
/// nested list, so it cannot flow inline. The marker paints in this gutter and
/// the item's children (label text and the nested list) are inset past it, so
/// nested lists indent one marker-width per level. Inline list items emit their
/// marker through the inline flow instead, and generated leaves like `<hr>`
/// paint it as their whole content, so both return `0`.
pub(super) fn marker_gutter(
	node: &CharcellNodeData,
	query: &CharcellQuery,
) -> u32 {
	if establishes_inline_flow(node, query) {
		return 0;
	}
	match node.marker() {
		Some(marker) if node.has_child_nodes(query) => {
			display_width(marker) as u32
		}
		_ => 0,
	}
}

/// Measure an inline formatting context: returns `(max_line_width, line_count)`.
pub(super) fn measure_inline_flow(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	available_width: u32,
) -> UVec2 {
	let mut runs = collect_inline_runs(node, query);
	// font-size scaling: above 2em the whole context is one block-font run;
	// above 1em every run is remapped to fullwidth and flows as usual.
	match FontScale::of_style(node.visual_style()) {
		FontScale::Block => {
			return measure_block_text(&inline_text(&runs), available_width);
		}
		FontScale::Wide => widen_runs(&mut runs),
		FontScale::Normal => {}
	}
	let lines = flow_inline(&runs, available_width, is_preformatted(node));
	let max_w = lines.iter().map(line_width).max().unwrap_or(0);
	UVec2::new(max_w, lines.len() as u32)
}

/// Paint an inline formatting context into `content_rect`, flowing the
/// container's descendant runs into wrapped, aligned lines.
pub(super) fn paint_inline_flow(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	content_rect: IRect,
	buffer: &mut impl AsBuffer,
	clip: Clip,
) {
	let mut runs = collect_inline_runs(node, query);
	// font-size scaling mirrors `measure_inline_flow`: paint the context as one
	// block-font run above 2em, or remap every run to fullwidth above 1em.
	match FontScale::of_style(node.visual_style()) {
		FontScale::Block => {
			let visual = node.visual_style();
			paint_block_text(
				&inline_text(&runs),
				content_rect,
				visual,
				visual.text_align,
				node.entity,
				buffer,
				clip,
			);
			return;
		}
		FontScale::Wide => widen_runs(&mut runs),
		FontScale::Normal => {}
	}
	let width = content_rect.width().max(0) as u32;
	let lines = flow_inline(&runs, width, is_preformatted(node));
	let align = node.visual_style().text_align;

	for (row, line) in lines.iter().enumerate() {
		let y = content_rect.min.y + row as i32;
		if y >= content_rect.max.y {
			break;
		}
		let mut x = content_rect.min.x
			+ align_offset(line_width(line), width, align) as i32;
		for span in line {
			if x >= content_rect.max.x {
				break;
			}
			let avail = (content_rect.max.x - x).max(0) as usize;
			let text = truncate_to_width(&span.text, avail);
			buffer.write_text(
				IVec2::new(x, y),
				text,
				span.style.clone(),
				span.entity,
				clip,
			);
			let span_width = display_width(text) as i32;
			// wrap the painted columns in this run's OSC-8 link (stdout only)
			if let Some(link) = &span.link {
				for col in x.max(0)..(x + span_width).max(0) {
					buffer.set_link(
						UVec2::new(col as u32, y.max(0) as u32),
						link,
					);
				}
			}
			x += span_width;
		}
	}
}

/// Depth-first collect every descendant text [`Value`] as a styled run, in
/// document order. Each run carries its own resolved style and source entity.
fn collect_inline_runs(
	node: &CharcellNodeData,
	query: &CharcellQuery,
) -> Vec<InlineRun> {
	let mut runs = Vec::new();
	collect_runs_inner(node, query, None, &mut runs);
	// descendant runs with no background of their own (eg syntax-highlighted
	// spans) inherit the IFC owner's fill, so a `<pre>` background paints behind
	// every run rather than leaving holes where coloured spans overwrite it.
	if let Some(background) = node.visual_style().background {
		for run in &mut runs {
			run.style.background.get_or_insert(background);
		}
	}
	runs
}

fn collect_runs_inner(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	link: Option<&str>,
	runs: &mut Vec<InlineRun>,
) {
	// an `<a>`/`<img>` descendant supplies the link for everything beneath it
	let link = node.hyperlink().or(link);
	// generated content (bullet, quote bar, alt text) leads the node's runs
	if let Some(marker) = node.marker().filter(|marker| !marker.is_empty()) {
		runs.push(InlineRun {
			text: marker.to_string(),
			style: node.visual_style().clone(),
			entity: node.entity,
			link: link.map(SmolStr::from),
		});
	}
	if let Some(value) = node.value() {
		let text = value.to_string();
		if !text.is_empty() {
			runs.push(InlineRun {
				text,
				style: node.visual_style().clone(),
				entity: node.entity,
				link: link.map(SmolStr::from),
			});
		}
	}
	for child in node.child_nodes(query) {
		collect_runs_inner(&child, query, link, runs);
	}
}

/// Concatenate run texts into the raw string the block font renders.
fn inline_text(runs: &[InlineRun]) -> String {
	runs.iter().map(|run| run.text.as_str()).collect()
}

/// Remap every run to fullwidth glyphs for the wide (> 1em) scale, leaving each
/// run's weight to the cascade — headings carry their user-agent bold, so wide
/// non-heading text stays plain rather than being forced bold.
fn widen_runs(runs: &mut [InlineRun]) {
	for run in runs.iter_mut() {
		run.text = to_fullwidth(&run.text);
	}
}

/// Flow `runs` into lines of styled spans, wrapping at `max_w` columns.
///
/// In preformatted mode whitespace and newlines are preserved verbatim;
/// otherwise whitespace is collapsed, lines wrap on word boundaries (hard
/// breaking words longer than the column), and `\n` forces a line break.
fn flow_inline(
	runs: &[InlineRun],
	max_w: u32,
	preformatted: bool,
) -> Vec<Vec<InlineSpan>> {
	// flatten into a (char, run index) stream so styles are cloned per span,
	// not per character.
	let mut chars: Vec<(char, usize)> = Vec::new();
	for (idx, run) in runs.iter().enumerate() {
		for ch in run.text.chars() {
			chars.push((ch, idx));
		}
	}

	let line_chars = if preformatted {
		split_pre_lines(&chars)
	} else {
		wrap_lines(&chars, max_w)
	};

	line_chars
		.iter()
		.map(|line| group_spans(line, runs))
		.collect()
}

/// Width of a tab stop, matching the web's `tab-size: 4` (Preflight).
const TAB_WIDTH: usize = 4;

/// Split the char stream into lines on `\n`, preserving everything else.
///
/// Tabs are expanded to spaces up to the next [`TAB_WIDTH`] stop, since a raw
/// `\t` left in a cell makes the terminal jump to its own tab stop and overflow
/// the code block's box (the web expands tabs via `tab-size`).
fn split_pre_lines(chars: &[(char, usize)]) -> Vec<Vec<(char, usize)>> {
	let mut lines = Vec::new();
	let mut current = Vec::new();
	let mut col = 0usize;
	for &(ch, idx) in chars {
		match ch {
			'\n' => {
				lines.push(core::mem::take(&mut current));
				col = 0;
			}
			'\t' => {
				let stop = (col / TAB_WIDTH + 1) * TAB_WIDTH;
				while col < stop {
					current.push((' ', idx));
					col += 1;
				}
			}
			_ => {
				current.push((ch, idx));
				col += unicode_width(ch) as usize;
			}
		}
	}
	lines.push(current);
	// drop a single trailing empty line: a fenced code block's text ends with a
	// `\n`, which would otherwise render an empty row inside the `<pre>` box and
	// gives it an uneven one-above / two-below gutter.
	if lines.len() > 1 && lines.last().is_some_and(|line| line.is_empty()) {
		lines.pop();
	}
	lines
}

/// Greedy word-wrap of the styled char stream at `max_w` columns, collapsing
/// whitespace and breaking on `\n`.
fn wrap_lines(chars: &[(char, usize)], max_w: u32) -> Vec<Vec<(char, usize)>> {
	let max_w = max_w as usize;
	if max_w == 0 {
		return vec![
			chars.iter().filter(|(c, _)| *c != '\n').copied().collect(),
		];
	}
	let mut lines = Vec::new();
	let mut cur: Vec<(char, usize)> = Vec::new();
	let mut cur_w = 0usize;
	// the collapsed-whitespace gap awaiting the next word: its space char (a
	// 2-cell `FULLWIDTH_SPACE` is preserved so fullwidth runs keep a wide gap)
	// and the style index it belongs to.
	let mut pending_space: Option<(char, usize)> = None;

	let mut i = 0;
	while i < chars.len() {
		let (ch, idx) = chars[i];
		// in normal flow all whitespace (including newlines) collapses to a
		// single inter-word gap; only `white-space: pre` preserves newlines.
		if ch.is_whitespace() {
			if !cur.is_empty() {
				let space = if ch == FULLWIDTH_SPACE { ch } else { ' ' };
				pending_space = Some((space, idx));
			}
			i += 1;
			continue;
		}
		// gather a word: a maximal run of non-whitespace characters
		let start = i;
		let mut word_w = 0usize;
		while i < chars.len() && !chars[i].0.is_whitespace() {
			word_w += unicode_width(chars[i].0) as usize;
			i += 1;
		}
		let word = &chars[start..i];
		let space_w =
			pending_space.map_or(0, |(c, _)| unicode_width(c) as usize);

		// wrap before the word if it would overflow the current line
		if !cur.is_empty() && cur_w + space_w + word_w > max_w {
			lines.push(core::mem::take(&mut cur));
			cur_w = 0;
			pending_space = None;
		}
		if let Some((space, space_idx)) = pending_space.take() {
			if !cur.is_empty() {
				cur.push((space, space_idx));
				cur_w += unicode_width(space) as usize;
			}
		}
		if word_w > max_w {
			// hard-break a word longer than the whole column
			for &(c, ci) in word {
				let cw = unicode_width(c) as usize;
				if !cur.is_empty() && cur_w + cw > max_w {
					lines.push(core::mem::take(&mut cur));
					cur_w = 0;
				}
				cur.push((c, ci));
				cur_w += cw;
			}
		} else {
			cur.extend_from_slice(word);
			cur_w += word_w;
		}
	}
	lines.push(cur);
	lines
}

/// Coalesce a line's `(char, run index)` pairs into [`InlineSpan`]s, merging
/// adjacent characters that share a run.
fn group_spans(line: &[(char, usize)], runs: &[InlineRun]) -> Vec<InlineSpan> {
	let mut spans: Vec<InlineSpan> = Vec::new();
	let mut last_idx: Option<usize> = None;
	for &(ch, idx) in line {
		if last_idx == Some(idx) {
			spans.last_mut().unwrap().text.push(ch);
		} else {
			spans.push(InlineSpan {
				text: ch.to_string(),
				style: runs[idx].style.clone(),
				entity: runs[idx].entity,
				link: runs[idx].link.clone(),
			});
			last_idx = Some(idx);
		}
	}
	spans
}

/// Total display width of a flowed line.
fn line_width(line: &Vec<InlineSpan>) -> u32 {
	line.iter()
		.map(|span| display_width(&span.text) as u32)
		.sum()
}

/// Leading-column offset for a line of `line_w` columns within `width`.
pub(super) fn align_offset(line_w: u32, width: u32, align: TextAlign) -> u32 {
	let pad = width.saturating_sub(line_w);
	match align {
		TextAlign::Left => 0,
		TextAlign::Right => pad,
		TextAlign::Center => pad / 2,
	}
}

/// Truncate `text` to at most `max_cols` display columns.
pub(super) fn truncate_to_width(text: &str, max_cols: usize) -> &str {
	let mut width = 0;
	for (i, ch) in text.char_indices() {
		let w = unicode_width(ch) as usize;
		if width + w > max_cols {
			return &text[..i];
		}
		width += w;
	}
	text
}

#[cfg(test)]
mod tests {
	use super::*;

	fn run(text: &str) -> InlineRun {
		InlineRun {
			text: text.to_string(),
			style: VisualStyle::default(),
			entity: Entity::PLACEHOLDER,
			link: None,
		}
	}

	/// Flatten flowed lines back to plain strings for assertions.
	fn lines_text(lines: &[Vec<InlineSpan>]) -> Vec<String> {
		lines
			.iter()
			.map(|line| {
				line.iter()
					.map(|span| span.text.as_str())
					.collect::<String>()
			})
			.collect()
	}

	#[beet_core::test]
	fn flows_runs_onto_one_line() {
		let runs = [run("Hello "), run("world"), run("!")];
		lines_text(&flow_inline(&runs, 40, false))
			.xpect_eq(vec!["Hello world!".to_string()]);
	}

	#[beet_core::test]
	fn wraps_at_column_width() {
		let runs = [run("one two three four")];
		// width 7 wraps after each ~word
		lines_text(&flow_inline(&runs, 7, false)).xpect_eq(vec![
			"one two".to_string(),
			"three".to_string(),
			"four".to_string(),
		]);
	}

	#[beet_core::test]
	fn collapses_whitespace_across_runs() {
		// trailing space in one run + leading space in next collapse to one
		let runs = [run("foo   "), run("   bar")];
		lines_text(&flow_inline(&runs, 40, false))
			.xpect_eq(vec!["foo bar".to_string()]);
	}

	#[beet_core::test]
	fn hard_breaks_overlong_word() {
		let runs = [run("abcdefghij")];
		lines_text(&flow_inline(&runs, 4, false)).xpect_eq(vec![
			"abcd".to_string(),
			"efgh".to_string(),
			"ij".to_string(),
		]);
	}

	#[beet_core::test]
	fn newline_collapses_to_space_in_normal_mode() {
		let runs = [run("a\nb")];
		lines_text(&flow_inline(&runs, 40, false))
			.xpect_eq(vec!["a b".to_string()]);
	}

	#[beet_core::test]
	fn preformatted_preserves_whitespace_and_newlines() {
		let runs = [run("fn  main()\n    body")];
		lines_text(&flow_inline(&runs, 4, true))
			.xpect_eq(vec!["fn  main()".to_string(), "    body".to_string()]);
	}

	#[beet_core::test]
	fn preformatted_expands_tabs_to_stops() {
		// tabs advance to the next 4-column stop rather than leaking a raw `\t`
		// that the terminal would re-expand and overflow the code block.
		let runs = [run("\tfn\tx")];
		lines_text(&flow_inline(&runs, 40, true))
			.xpect_eq(vec!["    fn  x".to_string()]);
	}

	#[beet_core::test]
	fn spans_keep_their_run_style() {
		let mut italic = VisualStyle::default();
		italic = italic.italic();
		let runs = [run("plain "), InlineRun {
			text: "fancy".to_string(),
			style: italic.clone(),
			entity: Entity::PLACEHOLDER,
			link: None,
		}];
		let lines = flow_inline(&runs, 40, false);
		// one line, two spans: "plain " (default) and "fancy" (italic)
		let line = &lines[0];
		line.len().xpect_eq(2);
		line[1].text.as_str().xpect_eq("fancy");
		line[1].style.clone().xpect_eq(italic);
	}
}

/// End-to-end pipeline tests: parse a tree, run the full charcell pipeline,
/// and assert the flowed plain-text output.
#[cfg(test)]
mod pipeline_tests {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::math::UVec2;

	fn render(size: UVec2, bundle: impl Bundle) -> String {
		Buffer::render_oneshot_plain_sized(size, bundle)
			.trim_lines()
			.lines()
			.map(|line| line.trim_end())
			.collect::<Vec<_>>()
			.join("\n")
	}

	#[beet_core::test]
	fn paragraph_flows_mixed_inline_children() {
		// text, emphasis and trailing text flow onto one continuous line
		render(
			UVec2::new(40, 5),
			rsx! { <p>"Hello "<em>"world"</em>"!"</p> },
		)
		.xpect_eq("Hello world!");
	}

	#[beet_core::test]
	fn paragraph_wraps_at_content_width() {
		render(UVec2::new(9, 5), rsx! { <p>"one two three four"</p> })
			.xpect_eq("one two\nthree\nfour");
	}

	#[beet_core::test]
	fn preformatted_preserves_newlines_and_spaces() {
		render(
			UVec2::new(20, 5),
			rsx! { <pre>"fn  main()\n    body"</pre> },
		)
		.xpect_eq("fn  main()\n    body");
	}

	#[beet_core::test]
	fn normal_paragraph_collapses_newlines() {
		// outside <pre>, an embedded newline collapses to a space in the flow
		render(UVec2::new(40, 5), rsx! { <p>"alpha\nbeta"</p> })
			.xpect_eq("alpha beta");
	}

	#[beet_core::test]
	fn paragraph_tail_survives_narrow_column() {
		use crate::prelude::style::*;
		// a paragraph laid out in a grow column narrower than its natural width
		// wraps into more rows than measured at the viewport width; its reserved
		// height must follow the assigned width so the tail is not clipped.
		let text = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
		let out = FlexBuffer::render_oneshot_plain(
			40,
			(LayoutStyle::flex_row(), children![
				(rsx! { "SIDEBARWIDTH" }, LayoutStyle::default()),
				(LayoutStyle::default().with_flex_grow(1), children![
					rsx! { <p>{text}</p> }
				],),
			]),
		);
		// every word, including the last, makes it into the flowed column
		out.as_str().xpect_contains("alpha").xpect_contains("kappa");
	}

	#[beet_core::test]
	fn unfilled_inline_slot_keeps_flow_inline() {
		// a trailing `{Option::None}` lowers to an empty placeholder node; it must
		// not break the paragraph's inline formatting context onto separate lines.
		let trailing: Option<_> = None::<&str>.map(|ancestor: &str| {
			rsx! { " more "<a href="/x">{ancestor}</a> }
		});
		FlexBuffer::render_oneshot_plain(
			40,
			rsx! { <p>"Route "<a href="/">"/"</a>" not found."{trailing}</p> },
		)
		.lines()
		.map(|line| line.trim_end())
		.filter(|line| !line.trim().is_empty())
		.collect::<Vec<_>>()
		.xpect_eq(vec!["Route / not found."]);
	}

	#[beet_core::test]
	fn nested_emphasis_combines_bold_and_italic() {
		// `<em><strong>` must resolve to both italic and bold via the
		// independently-inheriting font-style and font-weight cascades.
		Buffer::render_oneshot_sized(
			UVec2::new(40, 3),
			rsx! { <p><em><strong>"x"</strong></em></p> },
		)
		.xpect_contains("\x1b[1m")
		.xpect_contains("\x1b[3m");
	}
}

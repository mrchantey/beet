//! Office Open XML: Word files, workbooks and slide decks, read into and
//! written from the one document model.
//!
//! A reader is a [`DocParser`]: it reads a file into the entities markdown
//! and HTML read into, [`Element`]s, their attributes and [`Value`] text,
//! with what the file formats directly carried as style rules (see
//! [Style](#style)), and nothing Office-specific is left on any entity once
//! it finishes. A writer is a [`DocRenderer`]: it writes from the tree alone,
//! never consulting the file a tree was read from, so a markdown page writes
//! as a Word file and a deck authored as cards writes as a slide deck. Both
//! directions are lossy, and the tables below say what each keeps. The
//! acceptance test is the model's round trip, never the file's: a tree
//! written and read again is the same tree.
//!
//! | Format | Reader | Writer |
//! |---|---|---|
//! | Word | `WordParser` | `WordRenderer` |
//! | Workbook | `WorkbookParser` | `WorkbookRenderer` |
//! | Slide deck | `DeckParser` | `DeckRenderer` |
//!
//! Each format's table reads left to right as the read and right to left as
//! the write; a row whose write differs from its read says so.
//!
//! # Word
//!
//! The closest HTML analogy. The body's blocks are the root's children in
//! order, the section's default header first, the notes after the body and
//! the default footer last. Named styles are matched by their built-in
//! names through `styles.xml` (`heading 1`, `Title`, `Quote`), never their
//! ids, which Word localizes, ie `berschrift1`; a file with no styles part
//! is matched by the English ids.
//!
//! | Word | HTML | Notes |
//! |---|---|---|
//! | the body's last `w:sectPr`'s default `w:headerReference`, `w:footerReference` | the root's first child a `header`, its last a `footer`, each of the part's blocks | a write writes the root's first child as the header when it is a `header`, and its last as the footer when a `footer` |
//! | `w:p` | `p` | |
//! | `w:p` styled `Title`, `Heading1` to `Heading6` | `h1` to `h6` | `Title` reads as `h1`; a heading writes with its `Heading<n>` style, defined with its built-in name and outline level so the navigation pane and a contents page find it |
//! | consecutive `Quote` or `IntenseQuote` paragraphs | `blockquote` of `p`s | each `p` in a `blockquote` writes with `Quote` |
//! | consecutive `HTMLPreformatted` paragraphs | one `pre`, a line each, every line wholly `HTMLCode` read as one `code` | a `pre` writes a paragraph a line, so adjacent `pre`s read back as one |
//! | a run styled `HTMLCode` | `code` | |
//! | a `Caption` paragraph directly above a table | that table's `caption` | |
//! | a paragraph numbered by `w:numPr`, its own or its style's | `li`, grouped by `w:ilvl` into nested `ul`, or `ol` when the level's `w:numFmt` counts (decimal, letter, roman), a deeper level nested in the item before it, a top level item of another `w:num` starting a new list | a write numbers each `li` at its depth with the writer's own bullet or decimal definition, each top level list, and each nested list of the other kind, its own `w:num` restarting its count |
//! | an empty paragraph with only a bottom border | `hr` | |
//! | `w:b`, `w:i`, `w:u`, `w:strike` or `w:dstrike` | `strong`, `em`, `u`, `s`; turned off, ie `w:b w:val="0"`, a `span` with a direct look of its weight, slant or decoration | a write reads the resolved weight, slant and decoration, so any element styled bold writes bold, and one resolving less than its paragraph turns it off |
//! | `w:vertAlign` | `sup`, `sub` | |
//! | `w:highlight` | `mark`, a colour other than yellow as a direct look of its background | a `mark` with no resolved background writes yellow |
//! | `w:color` (not `auto`), `w:shd` fill, `w:sz` differing from its paragraph style's, else the document's default | `span` with a direct look of its colour, background and size | a run's look nests outermost first: `mark` or `span`, `strong`, `em`, `u`, `s`, `sub` or `sup`, `code` |
//! | a paragraph's `w:jc` (`center`, `right`) and `w:shd` fill | a direct look of its text alignment and background on the block | |
//! | `w:pageBreakBefore` | a `PageBreak` before the block | |
//! | `w:hyperlink r:id` | `a href` to its relationship's target | |
//! | `w:hyperlink w:anchor`, `w:bookmarkStart w:name` | `a href="#name"`, the `id` of the block the bookmark opens in | `_GoBack` is dropped; a write brackets any block carrying an `id` in a bookmark of that name |
//! | a complex field `HYPERLINK ".."` | `a href` | |
//! | any other field, simple or complex | its result as last computed | the instruction is lost |
//! | `w:br` | `br` | |
//! | `w:br w:type="page"` | `div class="page-break"`, beet's `PageBreak` | a write breaks the page after any element resolving `break-after: page` |
//! | `w:tab`, `w:sym`, `w:noBreakHyphen` | a space, its character, `U+2011` | |
//! | `w:sdt` holding a `w14:checkbox` | `input type="checkbox"`, `checked` as it says | its glyph is dropped; a write writes the control and its glyph, `☒` or `☐` |
//! | any other `w:sdt` | its content, the wrapper dropped | |
//! | `w:ins`, `w:moveTo`; `w:del`, `w:moveFrom` | `ins`; `del`, with `datetime` from `w:date` | a write's `w:author` is the document's first author, else `beet` |
//! | `w:tbl` | `table` | a write spans the page's width with single borders, a cell ending in a table closed by the empty paragraph Word requires, which a read drops |
//! | a cell holding one plain paragraph | the paragraph's content, its alignment and id the cell's | a cell's inline content writes as one paragraph |
//! | `w:tr` with `w:tblHeader` | a `tr` of `th`s in `thead`, every other row a `tr` of `td`s in `tbody` | a `th` outside a `thead` writes as a cell and reads back a `td` |
//! | `w:gridSpan`; `w:vMerge`; `w:gridBefore` | `colspan`; `rowspan` on the merge's first cell, its continuations absent; an empty cell per skipped column, so every slot keeps its address | |
//! | a cell's `w:shd` fill | a direct look of its background on the cell | |
//! | `w:drawing` of an embedded picture, inline or anchored, or a VML `v:imagedata` | `img src="data:<type>;base64,.."` with `alt` from its description and `width` and `height` in CSS px | an anchored picture reads inline where it is anchored; see [Pictures](#pictures) |
//! | a paragraph holding only a text box, `w:txbxContent` | `aside` | a text box beside other content reads as an `aside` after its paragraph; a write writes an `aside` as an inline text box of its blocks |
//! | `w:footnoteReference`, `w:endnoteReference` | `sup` holding `a href="#fn-<n>"` or `#en-<n>`, `<n>` counting each kind's notes in reference order | the markdown parser's footnote shape; a write numbers each kind in reference order, a `sup` holding only a link to a definition being a reference |
//! | the notes parts | each note referenced a `div class="footnote-definition" id="fn-<n>"` (`en-<n>` for an endnote) after the body, its mark and the space after it dropped | a definition whose label leads `en-` writes as an endnote, any other as a footnote, one never referenced not at all |
//! | `mc:AlternateContent` | its `mc:Choice` when the reader reads what it requires (a text box, a checkbox), else its `mc:Fallback` | |
//!
//! **Lost on a read**: named styles beyond those listed and every style's own
//! formatting, theme fonts, page size, margins, columns and section breaks,
//! first page and even page headers and footers, comments, revision
//! authorship, field instructions, content control tags and bindings, a
//! tab's width, paragraph spacing and indents, justified alignment, table and
//! cell widths, borders and vertical alignment, every bookmark of a block but
//! its first, and where a floating picture or text box sits.
//!
//! **A write cannot say**: layout (flex rows and columns, positioning,
//! widths other than a picture's), a `th` outside a `thead`, form controls
//! other than a checkbox (an `input` writes its value), `video`, `iframe`
//! and inline `svg` (their fallback text), and the grouping of `section`,
//! `article`, `nav`, `main`, `div` and `dl`, whose blocks write in order. A
//! write is A4 portrait with one inch margins.
//!
//! # Workbook
//!
//! | Workbook | HTML | Notes |
//! |---|---|---|
//! | the workbook | the root, holding one `article data-worksheet="<tab name>"` per worksheet in tab order | a chartsheet is skipped |
//! | a worksheet hidden by `state` | the article's `hidden` | |
//! | a worksheet's cells | the article's one `table`, its rows `tr`s of `td`s in a `tbody`, dense from `A1` to the last used cell | see [Padding](#padding) |
//! | a cell's value | its text: a string resolved from the shared strings, a number as written, a boolean `TRUE` or `FALSE`, an error as its code, ie `#DIV/0!` | |
//! | a formula `f` | `data-formula="=SUM(A1:A3)"` on the cell, its text the value last computed | a shared formula's dependents read with its references offset, an array formula on its first cell only |
//! | a merged range | `colspan` and `rowspan` on its first cell, the cells it covers absent | |
//! | a locked cell on a protected sheet | `contenteditable="false"` on the cell | a write protects a sheet holding any locked cell, and unlocks every other cell of it |
//! | a cell's bold, italic, underline or strike, font colour, solid fill, horizontal alignment other than left, and font size, each where its cell format differs from the workbook's default format | a direct look on the cell | the default format is the root's look on a write, so a read never carries it as a direct look; a write reads the resolved style and writes each distinct look as one cell format |
//! | a cell's hyperlink | an `a href` holding the cell's text | |
//!
//! A cell's address is its slot in the HTML table, never an attribute, as
//! HTML's table model counts them, spans included, which the table grid
//! query (`TableGrid`) answers for any table whatever it was read from. A
//! workbook names a slot by its A1 reference (`A1Reference`), so `B3` is the
//! grid's `UVec2::new(1, 2)`, the second column of the third row. HTML has
//! no word for a worksheet's name nor for a formula, so a worksheet is a
//! `data-worksheet` and a formula a `data-formula`, the latter until beet's
//! scripting gives formulas a home.
//!
//! **Lost on a read**: number and date formats (a value reads as stored, so
//! a date reads as its serial number), rich text runs within a cell, column
//! widths and row heights, borders, frozen panes, conditional formats, data
//! validation, defined names, comments, charts and drawings.
//!
//! **A write**: every table not nested in another, in document order, writes
//! as a sheet, so any document holding tables writes as a workbook and one
//! holding none is refused. A sheet is named by its enclosing
//! `article data-worksheet`, else its `caption`'s text, else `Sheet<n>` by
//! its place among the sheets written, made valid (at most 31 characters,
//! none of `[]:*?/\`) and unique. A cell's text is typed as a person typing
//! it would: a number when the text is that number's canonical decimal form,
//! so `007` stays a string, `TRUE` or `FALSE` a boolean, and a
//! `data-formula` a formula whose cached value is typed the same way, an
//! error code as an error. A nested table writes as its text, a `thead` as
//! plain rows, and the workbook is marked to recalculate on open, since
//! nothing here computes.
//!
//! ## Padding
//!
//! A read pads every row to the sheet's last used column and every sheet to
//! its last used row, so every position is an address. A used cell is one
//! holding a value or a formula, the last corner of a merged range, or an
//! unlocked cell on a protected sheet (a form's field); a cell holding only
//! a style is not, nor is a column's style. A sheet whose padded area,
//! columns times rows, exceeds `WorkbookParser::MAX_SHEET_CELLS` (100 000),
//! or a workbook whose sheets together exceed
//! `WorkbookParser::MAX_WORKBOOK_CELLS` (1 000 000), is refused with an error
//! naming the sheet, its last used cell and the area it would pad to, so a
//! stray cell at `XFD1048576` costs an error, never the program's memory.
//!
//! # Slide deck
//!
//! A deck reads as the card stack reads a card (`crates/beet_router` card
//! stack): a slide is an `article data-card` holding its front, then an
//! `hr`, then its speaker notes. HTML has no word for an article that is a
//! card rather than a piece of a card's content, so `data-card` marks it,
//! and an `article` without it is content. A document of card articles
//! renders whole as one page, and a card deck steps through a route's card
//! articles before moving to its next route, a route holding none being one
//! card, so a deck mixes routes of one card with routes of several. Markdown
//! authors several cards in one file the same way, each an
//! `<article data-card>` block holding its markdown, its notes after its
//! `---`.
//!
//! Placeholders are matched by type: `ctrTitle` and `title` a title,
//! `subTitle` a subtitle, `body`, `obj` and an untyped placeholder a body,
//! and the date, footer, slide number and header placeholders the layout's
//! chrome, never read. A placeholder with no frame of its own takes its
//! layout's, else its master's, matched by `idx`, else by type.
//!
//! | Deck | HTML | Notes |
//! |---|---|---|
//! | the presentation's slide list | the root's `article data-card`s in presentation order | a deleted slide's leftover part is skipped |
//! | a hidden slide, `show="0"` | the article's `hidden` | |
//! | a slide of a title and only subtitles beside it | `div class="card-center"`: the title an `h1`, each subtitle paragraph a `p` | writes with the Title Slide layout |
//! | a slide with a title and anything else | `div class="card-content"`: `header class="card-heading"` holding the title as an `h1`, then `div class="card-columns"` of `div class="card-column"`s | one column, or one per body placeholder when two or more sit side by side, each other shape joining the column it overlaps most; writes with the Title and Content layout, Two Content for two columns or more |
//! | a slide with no title | its shapes' blocks in reading order, no layout wrapper | writes with the Title and Content layout, a leading heading as its title |
//! | shapes | flattened in reading order, top to bottom then left to right, a group's shapes in its place | positions and sizes are lost, a picture's size kept; an empty shape reads as nothing |
//! | a title's paragraphs | one `h1`, a `br` between each | |
//! | a body placeholder's paragraphs | `ul` of `li`s nested by `lvl`, `ol` for `a:buAutoNum`, `p` for `a:buNone`, a paragraph setting no bullet taking its shape's list style, its layout's or master's placeholder's, else the master's body style | a write leaves a list item to the master's bullets, a counted one `a:buAutoNum`, and a `p` `a:buNone` |
//! | any other text frame's paragraphs | `p`, a list only where a bullet is set on it or its list styles | |
//! | a paragraph's `algn` (`ctr`, `r`) | a direct look of its text alignment | |
//! | `a:r` with `b`, `i`, `u`, `strike`, `baseline` | `strong`, `em`, `u`, `s`, `sup` or `sub`; a toggle turned off is the master's to decide and reads as nothing | a write reads the resolved weight, slant and decoration, as a Word run writes |
//! | a run's solid fill, `a:highlight`, and `sz` where it differs from the size most of its frame's text is set in | a `span` with a direct look of its colour, background and size | |
//! | `a:hlinkClick` | `a href` | |
//! | `a:br`, `a:fld` | `br`, the field's text as last computed | |
//! | `a:tbl` | `table`, its first row a `thead` of `th`s when `firstRow` says so, `gridSpan` and `rowSpan` as `colspan` and `rowspan`, the cells `hMerge` and `vMerge` cover absent, a cell's solid fill a direct look of its background | |
//! | `p:pic` | `img`, as a Word picture reads, sized by its frame | |
//! | SmartArt | its data part's paragraphs as `p`s | its structure is lost and a write cannot make one |
//! | `mc:AlternateContent` | its `mc:Choice` when it reads anything, else its `mc:Fallback` | |
//! | the notes slide's body placeholder | the article's content after its `hr` | |
//!
//! **Lost on a read**: masters, theme fonts, backgrounds, transitions and
//! animations, charts, connectors, shape geometry, every position, and
//! the empty placeholders a layout leaves.
//!
//! **A write**: each `article data-card` writes as a slide, or the whole
//! document as one slide when it holds none. A card's front writes through
//! its layout as above: the title placeholder takes the heading, each column
//! a body placeholder of its blocks (headings below the title as bold
//! paragraphs, lists as bulleted levels, a `pre` a monospace line each, and
//! the grouping of a `blockquote`, `div` or `section` lost), and every
//! picture and table its own shape, stacked below the text in flow order:
//! the body frame shrinks to its text's estimated height (a glyph half its
//! size wide, a line 1.2 times its size) so each picture and table fits what
//! remains of its column at its aspect. The content after the article's
//! first `hr` child writes as its speaker notes, and `hidden` as a hidden
//! slide. A deck is 16:9, 13.333 by 7.5 inches (1280 by 720 px), with the
//! writer's own master, theme and three layouts and a notes master. A card
//! deck writes its whole stack as one deck, a route of no card articles one
//! slide (the card stack's module docs).
//!
//! # Style
//!
//! Every look a format reads or writes passes through the cascade, so a
//! document looks the same in a file, a terminal and a browser. A writer
//! writes what the world's rules resolve, never a stylesheet of its own, and
//! where nothing resolves the format's own default applies. HTML's user
//! agent defaults are beet's default element rules (`strong` bold, `em`
//! italic, `u` and `ins` underlined, `s` and `del` struck, no colours), so
//! the plugin installing the formats installs `StylePlugin`; colours come
//! from a theme when the world has one, and from a file's direct looks.
//!
//! ## Reading
//!
//! - **Direct looks.** What a file formats directly and no element says, ie
//!   a colour, a fill, a size or an alignment, a reader registers as a typed
//!   inline rule on the element, as `inline_class!` and `bx:style` register
//!   theirs, keyed by its declarations so equal looks share one class. The
//!   rule sits in the cascade's `Direct` origin, which outranks every author
//!   rule as an HTML `style` attribute does, so the terminal, the web's
//!   stylesheet and a writer all see the file's look.
//! - **Theme colours.** A colour a file names by its theme's slot (Word's
//!   `w:themeColor`, a workbook's `theme` index, DrawingML's `a:schemeClr`)
//!   reads as the token the slot maps to, so a document takes the world's
//!   theme and scheme as a web page does. A slot modified by a tint, shade
//!   or luminance reads as the colour it computes to, since no token names
//!   it. With no theme the token resolves nothing and the target's default
//!   colour shows.
//!
//! | Theme slot | Token |
//! |---|---|
//! | `dk1`, `tx1` | `OnSurface` |
//! | `lt1`, `bg1` | `Surface` |
//! | `dk2`, `tx2` | `OnSurfaceVariant` |
//! | `lt2`, `bg2` | `SurfaceContainer` |
//! | `accent1`, `hlink` | `Primary` |
//! | `accent2` | `Secondary` |
//! | `accent3`, `folHlink` | `Tertiary` |
//! | `accent4` | `PrimaryContainer` |
//! | `accent5` | `SecondaryContainer` |
//! | `accent6` | `TertiaryContainer` |
//!
//! - **Options.** Every reader holds an `OfficeReadOptions`:
//!   `direct_formatting` is `Override` (the default, the `Direct` origin),
//!   `Cascade` (the author origin at class specificity, so a more specific
//!   theme rule wins) or `Drop` (not read, the theme alone styling the
//!   document); `theme_colors` is `Tokens` (the default, by the table) or
//!   `Verbatim` (every colour as it computes in the file's own theme).
//!
//! ## Writing
//!
//! - **Scheme and medium.** A writer resolves for the request's
//!   `color-scheme` param, else the theme's own scheme, as a web page does,
//!   so a dark themed site writes a dark document. Its medium is `Print` for
//!   a Word file and `Screen` for a deck or a workbook, so a `Print` rule
//!   shapes a Word file and a `Terminal` one never does. A width gated rule
//!   sees the page's content box: 602 px for a Word file (A4, one inch
//!   margins), the slide's 1280 by 720 px for a deck, none for a workbook,
//!   which skips width rules. A writer runs `PostParseTree` first, then
//!   resolves through the cascade's pure resolve with that scheme, medium
//!   and viewport, never reading nor writing the components a live surface
//!   keeps, since the same tree may be on screen as it writes.
//! - **Where a look writes.** The root's resolved look (font size, colour,
//!   background) writes as the format's document defaults: Word's
//!   `w:docDefaults` and page background, a deck master's background and
//!   text styles, a workbook's default cell format. An element the format
//!   names with a style (`h1` to `h6` as `Heading<n>`, `blockquote` as
//!   `Quote`, `pre` as `HTMLPreformatted`, `code` as `HTMLCode`, `a` as
//!   `Hyperlink`, `caption` as `Caption`, a deck's title and body text)
//!   writes that style with the look its tag resolves to as a child of the
//!   root, a Word heading bold at Word's own sizes where nothing resolves it
//!   a weight or size of its own. A run, paragraph, cell or shape then
//!   writes as direct formatting only what it resolves differently from the
//!   structure a reader reads it back into, resolved as a probe of bare
//!   elements under the root: its style's tag inside the lists, quote or
//!   table around it, and the inline elements the format writes as
//!   structure, ie a link, a mark, code or a revision. A theme's palette and
//!   prose rules so land where the format keeps a document's look, and a
//!   written file read back resolves the same look through the same rules
//!   without carrying it as direct looks.
//! - **Theme colours.** A writer writes the format's theme colour scheme by
//!   the slot table, each slot its token resolved for the writer's scheme,
//!   and where the world's theme resolved those tokens, a colour it writes
//!   that equals a slot's writes as a reference to the slot, the first slot
//!   in the table winning. With no theme the scheme is Office's default and
//!   no colour names a slot, since its token would resolve nothing on a read.
//! - **Units.** px and rem by CSS's own ratios: 96 px to the inch, 72 pt to
//!   the inch, 16 px to the rem, so 1 px is 0.75 pt, 15 twips and 9525 EMU.
//!   A percentage width is of the page's content box.
//! - **What writes.** Run: weight, slant, decoration line, colour,
//!   background, font size, and the `sup`, `sub` and `code` elements (a
//!   monospace font), a `mark`'s background as the nearest highlight colour.
//!   Paragraph: text alignment, margins as spacing before, after and indent,
//!   background as shading, `break-after: page` (and beet's `PageBreak`
//!   markup, whatever it resolves). Cell: background, text alignment, and
//!   for a workbook the run properties.
//!   Shape: a slide's runs as a Word run's but code, which writes in a
//!   monospace font a read loses, a paragraph's alignment, a table cell's
//!   background, a picture's size.
//!
//! # Pictures
//!
//! A read embeds a picture's bytes in its `src` as a `data:` URL of its own
//! media type, so the tree holds everything it shows and no read does IO;
//! a linked picture's `src` is its link. A write embeds the bytes a
//! picture carries, a `data:` URL's or the
//! [`InlineBlob`](crate::prelude::InlineBlob) a media resolve step left on
//! its `src` (a served page fetches the site's own pictures for an Office
//! request), as an image part, sized by its resolved width and height, else
//! its `width` and `height` attributes, else its native size, within the
//! page's content box; any other `src` writes as a linked picture, since a
//! writer does no IO, which an Office reader shows when it can reach the
//! address.
//!
//! # Package
//!
//! `OpcPackage` reads and writes the zip package itself, its content types,
//! relationships and parts by path, over the `zip` crate with deflate
//! through the workspace's pure rust `flate2`. A reader parses a part
//! through the markup parser's XML dialect (`OpcPackage::xml`), and a writer
//! builds one as an `XmlPart`, escaping as [`XmlRenderer`] does.
//!
//! # Document properties
//!
//! A file's core properties, `docProps/core.xml`, read into the root's
//! [`DocMeta`] and write from it:
//!
//! | Core property | [`DocMeta`] |
//! |---|---|
//! | `dc:title` | `title` |
//! | `dc:description` | `description` |
//! | `dc:creator` | `authors`, split on `;` and joined by `; ` |
//! | `cp:keywords` | `tags`, split on `,` and `;`, each lowercase kebab |
//! | `dcterms:created` | `created`, the day |
//! | `dcterms:modified` | `updated`, the day |
//!
//! Dropped: the time of day, `dc:subject`, `cp:category`,
//! `cp:contentStatus`, `dc:identifier`, `dc:language`, `cp:version`,
//! `cp:lastModifiedBy`, `cp:revision` and `cp:lastPrinted`. `DocMeta`'s
//! labels, image and video have no property and are not written.
//!
//! [`DocParser`]: crate::prelude::DocParser
//! [`DocRenderer`]: crate::prelude::DocRenderer
//! [`DocMeta`]: crate::prelude::DocMeta
//! [`XmlRenderer`]: crate::prelude::XmlRenderer
//! [`Element`]: beet_core::prelude::Element
//! [`Value`]: beet_core::prelude::Value
mod a1_reference;
mod deck_parser;
mod deck_renderer;
mod doc_meta_properties;
mod namespace;
mod office_picture;
mod office_style;
#[cfg(test)]
mod office_test;
mod ooxml_plugin;
mod opc_package;
mod read_element;
#[cfg(any(test, feature = "testing"))]
mod word_fixture;
mod word_parser;
mod word_renderer;
mod workbook_parser;
mod workbook_renderer;
mod xml_part;
pub use a1_reference::*;
pub use deck_parser::*;
pub use deck_renderer::*;
pub use namespace::*;
pub use office_style::*;
pub use ooxml_plugin::*;
pub use opc_package::*;
#[cfg(any(test, feature = "testing"))]
pub use word_fixture::*;
pub use word_parser::*;
pub use word_renderer::*;
pub use workbook_parser::*;
pub use workbook_renderer::*;

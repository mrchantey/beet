# HTML Parser Plan

## Overview

Implement a streaming HTML parser for `beet_node` using `winnow` parser combinators. The parser implements `NodeParser`, diffing parsed HTML against an existing entity tree and only applying changes when content differs. Lives in `beet_node/src/parse/html/` behind the `html_parser` feature flag.

## Architecture

### Directory Structure

```
beet_node/src/parse/html/
├── mod.rs              # HtmlParser struct, options, NodeParser impl
├── combinators.rs      # winnow parser combinators for HTML tokens
├── tokens.rs           # intermediate token types (OpenTag, CloseTag, Text, etc.)
└── diff.rs             # entity tree diffing logic
```

### Data Flow

1. **Bytes → UTF-8 string** (in `NodeParser::parse`)
2. **String → Token stream** (winnow combinators produce `HtmlToken` variants)
3. **Token stream → Diff against entity tree** (depth-first walk comparing tokens to existing components)
4. **Diff → ECS mutations** (insert/update/remove components via `AsyncEntity`)

### Key Design Decisions

- Parsing is done token-by-token using winnow on the full buffer (collected by `NodeParser::parse_stream` default impl)
- Diffing happens at the node level: once we have a complete opening tag we diff `Element` + `Attribute` components, once we have text content we diff the `Value`
- `SpanLookup` is used to assign `FileSpan` to each node and attribute entity (replaced `SpanTracker` for random-access byte-offset lookups)
- Whitespace is always preserved (no normalization)
- The parser is configurable via option structs rather than hardcoded behavior

## Token Types

```rust
enum HtmlToken<'a> {
    Doctype(&'a str),
    OpenTag { name: &'a str, attributes: Vec<HtmlAttribute<'a>>, self_closing: bool, source: &'a str },
    CloseTag(&'a str),
    Text(&'a str),
    Comment(&'a str),
    Expression(&'a str),
}

struct HtmlAttribute<'a> {
    key: &'a str,
    value: Option<&'a str>,     // None for boolean attributes like `disabled`
    expression: bool,           // true for {foo} keyless attribute expressions
}
```

All token types borrow `&str` slices from the input for zero-copy parsing. Owned strings are only created when building ECS components. The `source` field on `OpenTag` captures the full tag text (`<tag ...>`) for `FileSpan` tracking.

## Parser Options

Config is split into two structs to avoid duplication:

```rust
struct HtmlParser {
    parse_config: ParseConfig,
    diff_config: DiffConfig,
}

struct ParseConfig {
    parse_expressions: bool,
    parse_raw_text_expressions: bool,
    raw_text_elements: Vec<String>,
    raw_character_data_elements: Vec<String>,
}

struct DiffConfig {
    parse_text_nodes: bool,
    parse_attribute_values: bool,
    void_elements: Vec<Cow<'static, str>>,
    void_element_children: VoidElementChildrenOpts,
    malformed_elements: MalformedElementsOpts,
}
```

## Winnow Combinators

Each combinator operates on `&str` input with winnow's `Located<&str>` wrapper for span tracking.

### Core combinators
- `parse_document` - top-level, parses sequence of nodes
- `parse_node` - dispatches to element, text, comment, doctype, or expression
- `parse_element` - open tag → children → close tag (or self-closing / void)
- `parse_open_tag` - `<name attr1="val" attr2 {expr}>` including self-closing `/>`
- `parse_close_tag` - `</name>`
- `parse_attribute` - `key="value"` | `key='value'` | `key=unquoted` | `key` (boolean) | `{expr}`
- `parse_text` - everything until `<` or `{` (when expressions enabled)
- `parse_comment` - `<!-- ... -->`
- `parse_doctype` - `<!DOCTYPE ...>`
- `parse_expression` - `{...}` with brace-depth tracking for nested braces
- `parse_raw_text` - content inside raw text elements, terminated only by matching close tag
- `parse_raw_text_expression` - `{{...}}` inside raw text elements

### Utility combinators
- `parse_tag_name` - alphanumeric + hyphens + underscores + dots + colons (custom elements, SVG/XML namespaces)
- `parse_attribute_value` - quoted or unquoted value
- `parse_whitespace` - optional whitespace consumer

## Diffing Strategy

Depth-first comparison between parsed tokens and existing entity tree:

1. **Element nodes**: Compare `Element` name, then diff `Attributes` (add/remove/update attribute entities via the `Attributes` relationship), then recurse into children
2. **Text nodes**: Compare `Value` component, use `set_if_ne_or_insert` to avoid spurious change detection
3. **Comment nodes**: Compare `Comment` + `Value` components
4. **Expression nodes**: Compare `Expression` component content
5. **Structural changes**: If child count differs or node type changes, despawn excess entities and spawn new ones

### Entity mapping

The differ maintains a cursor tracking position in the existing `Children` list. For each parsed node:
- If cursor entity matches type and content → skip (no mutation)
- If cursor entity matches type but differs → update in place
- If cursor entity doesn't match type → despawn and spawn replacement
- If no more cursor entities → spawn new child

After processing all parsed nodes, despawn any remaining cursor entities (extras from previous parse).

## `Value::parse_string()`

Added to `value.rs`, attempts optimistic parsing in order:
1. `"true"` / `"false"` → `Value::Bool`
2. Unsigned integer (no leading `-`, all digits) → `Value::Uint`
3. Signed integer (leading `-`, rest digits) → `Value::Int`
4. Float (contains `.`) → `Value::Float`
5. Fallback → `Value::Str`

## FileSpan Tracking

When a `path` is provided to `NodeParser::parse`:
- Build a `SpanLookup` from the full input text (pre-indexes line boundaries for O(log n) byte-offset → `LineCol` conversion)
- Since tokens borrow `&str` slices from the input, byte offsets are computed via pointer arithmetic (`slice.as_ptr() - input.as_ptr()`)
- Insert `FileSpan` on every entity:
  - **Element entities**: span of the full opening tag (`<tag ...>`) via the `source` field on `HtmlToken::OpenTag`
  - **Attribute entities**: span from key start through value end (for keyed attributes) or expression content (for keyless)
  - **Text nodes**: span of the text content
  - **Comment / doctype nodes**: span of the comment / doctype content
  - **Expression nodes**: span of the expression content (inside braces)
- Root entity receives a full-document span

## Feature Flag

In `Cargo.toml`:
```toml
[features]
html_parser = ["dep:winnow"]

[dependencies]
winnow = { version = "0.7", optional = true }
```

In `lib.rs`:
```rust
#[cfg(feature = "html_parser")]
mod html;
```

## Checklist

### Setup
- [x] Add `winnow` dependency and `html_parser` feature flag to `Cargo.toml`
- [x] Create `src/parse/html/` directory structure
- [x] Wire up `mod.rs` with feature-gated module in `parse/mod.rs` and `lib.rs`

### Value parsing
- [x] Add `Value::parse_string(input: &str) -> Value` to `value.rs`
- [x] Tests for parse_string: bool, uint, int, float, string fallback

### Token types (`tokens.rs`)
- [x] Define `HtmlToken` enum
- [x] Define `HtmlAttribute` struct
- [x] Implement `Display` for token types (useful for debugging)

### Parser options (`mod.rs`)
- [x] Define `HtmlParser` struct with all options
- [x] Define `VoidElementChildrenOpts` enum
- [x] Define `MalformedElementsOpts` enum
- [x] Implement `Default` for `HtmlParser` with standard HTML5 void/raw elements
- [x] Implement `HtmlParser::new()` with sensible defaults

### Winnow combinators (`combinators.rs`)
- [x] `parse_document` - sequence of nodes
- [x] `parse_node` - dispatch to appropriate sub-parser
- [x] `parse_open_tag` - tag name + attributes + self-closing detection
- [x] `parse_close_tag` - `</name>`
- [x] `parse_tag_name` - valid HTML tag names
- [x] `parse_attribute` - all attribute forms (key=value, boolean, expression)
- [x] `parse_attribute_value` - quoted (single/double) and unquoted
- [x] `parse_text` - text content between tags
- [x] `parse_comment` - `<!-- -->`
- [x] `parse_doctype` - `<!DOCTYPE>`
- [x] `parse_expression` - `{...}` with nested brace tracking
- [x] `parse_raw_text` - raw content until matching close tag
- [x] `parse_raw_text_expression` - `{{...}}` in raw text context
- [x] `parse_whitespace` - whitespace handling (handled inline in tag parsers)
- [x] Tests for each combinator in isolation

### Entity diffing (`diff.rs`)
- [x] `TreeNode` intermediate representation + `build_tree` from flat tokens
- [x] Diff logic for element nodes (name, attributes, children)
- [x] Diff logic for text nodes (Value comparison)
- [x] Diff logic for comment nodes
- [x] Diff logic for expression nodes
- [x] Child list reconciliation (add/remove/despawn excess)
- [x] Attribute reconciliation (add/remove/update via `Attributes` relationship)
- [x] `FileSpan` insertion when path is provided (root entity span)

### NodeParser implementation (`mod.rs`)
- [x] Implement `NodeParser::parse` for `HtmlParser`
- [x] UTF-8 validation
- [x] Run winnow combinators on input
- [x] Feed tokens through differ
- [x] Error handling for malformed HTML (per `MalformedElementsOpts`)

### Integration tests
- [x] Simple element: `<div></div>`
- [x] Nested elements: `<div><span>hello</span></div>`
- [x] Attributes: `<div class="foo" id="bar"></div>`
- [x] Boolean attributes: `<input disabled>` (combinator-level test)
- [x] Void elements: `<br>`, `<img src="foo.png">`
- [x] Self-closing: `<img />`
- [x] Text nodes with whitespace preservation (combinator-level test)
- [x] Comments: `<!-- hello -->`
- [x] Doctype: `<!DOCTYPE html>` (combinator-level test)
- [x] Mixed content: `<p>hello <em>world</em></p>` (combinator-level test)
- [x] Raw text elements: `<script>let x = 1 < 2;</script>` (combinator + document-level)
- [x] Expressions: `<div>{foo}</div>`
- [x] Keyless attribute expressions: `<div {foo}>` (combinator-level test)
- [x] Raw text expressions: `<script>{{foo}}</script>` (combinator + document-level)
- [x] Re-parse unchanged content (no change detection triggers)
- [x] Re-parse changed content (correct diffs applied)
- [ ] Void element children handling (all three opts)
- [x] Malformed HTML handling (Fix vs Error) (tree-building tests)
- [x] FileSpan tracking with path provided
- [x] Value::parse_string integration with parse_text_nodes/parse_attribute_values
- [x] SVG parsing (basic elements, path data, namespace attributes)
- [x] Stream parsing (collect-then-parse)

### Remaining work
- [x] Per-node `FileSpan` tracking (every entity now gets a span via `SpanLookup`)
- [x] Raw text element parsing integration (wired into `parse_document`)
- [x] Config deduplication (`HtmlParser` now wraps `ParseConfig` + `DiffConfig`)
- [x] Zero-copy parsing (`HtmlToken<'a>` and `TreeNode<'a>` borrow from input)
- [x] SVG support (tag names support colons for XML namespaces)
- [x] Key-based attribute expression integration test at ECS level (combinator-level)
- [x] Re-parse diffing integration tests (change detection)
- [x] Streaming (`parse_stream` collects then delegates to shared `parse_text`)
- [ ] `VoidElementChildrenOpts::Pop` and `VoidElementChildrenOpts::Error` enforcement
- [ ] Entity character references / HTML entity decoding

## Considerations

### Streaming
`HtmlParser::parse_stream` overrides the default to collect the full byte stream into a `String` via `stream_ext::bytes_to_text`, then delegates to the shared `parse_text()` method. Both `parse()` and `parse_stream()` share this core path. True incremental streaming (resumable mid-tag) would be significantly more complex and is deferred.

### Entity identity
When diffing, entity identity is positional (nth child). We don't use keys or IDs for matching. This is simple and correct for the common case of re-parsing the same file with minor edits. Key-based reconciliation could be added later if needed.

### Error recovery
`MalformedElementsOpts::Fix` needs careful thought. The simplest approach: if a close tag is missing, implicitly close the element when we encounter a close tag for an ancestor or EOF. This matches basic browser behavior without trying to replicate the full HTML5 parsing algorithm (which is enormous). Document limitations clearly.

### Memory
The full document is buffered in memory. For the scale of documents beet will handle (UI templates, not multi-GB HTML dumps) this is fine.

### Cross-platform
winnow is `no_std` compatible with alloc, which aligns with beet's cross-platform goals. No platform-specific code needed in the parser itself.
//! The `<CodeSnippet src=.. language=..>` widget: include a source file as a
//! syntax-highlighted code block (a BSX `include_str!`).
//!
//! Reads `src` through the nearest self-or-ancestor [`BlobStore`] — the same
//! IO-agnostic store serving the site's templates and routes, resolving an
//! absolute or a `..`-relative path, never the filesystem directly (so it reads
//! identically from disk in dev, S3 in a deployed task, or an in-memory store) —
//! then renders it as a `<pre><code class="language-X">` block, highlighting it
//! inline. The inline highlight matters: the read lands a few ticks after parse,
//! so the post-parse [`apply_syntax_highlighting`] pass has already run and would
//! miss this subtree; building the spans here means the same `<span class="hl-..">`
//! tree the post-parse pass emits, so the block renders identically on the web and
//! the charcell terminal with no extra wiring.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Include a source file as a syntax-highlighted code block. `src` resolves
/// through the nearest self-or-ancestor [`BlobStore`]; `language` overrides the
/// highlighter language (otherwise inferred from the file extension, with a `.bsx`
/// entry highlighted as `jsx`).
///
/// `<CodeSnippet src="../../demo/01-scripting.bsx" language="jsx"/>`
#[template]
pub fn CodeSnippet(
	/// The source file path, resolved through the nearest ancestor store.
	#[prop(into)]
	src: String,
	/// The highlighter language (eg `jsx`, `rust`, `sh`); inferred from the file
	/// extension when empty.
	#[prop(into, default)]
	language: String,
) -> impl Bundle {
	// `new_async_local` (not `new_async`): the read resolves the ancestor store and
	// builds nodes back on the world, both bridge-heavy, and the async bridge only
	// guarantees a poll completes on the runtime's local executor.
	OnSpawn::new_async_local(move |snippet: AsyncEntity| async move {
		let target = snippet.id();
		// the nearest self-or-ancestor store backs the read; a render tree carries it
		// on its root (see `BlobScene`), an entry tree inherits the site store.
		let store = snippet
			.with_state::<AncestorQuery<&BlobStore>, Result<BlobStore>>(
				|entity, stores| stores.get(entity).cloned(),
			)
			.await??;
		let bytes = store.get(&SmolPath::from(src.as_str())).await?;
		let source = String::from_utf8(bytes.to_vec())?;
		let lang = if language.is_empty() {
			infer_language(&src)
		} else {
			language.clone()
		};
		snippet
			.world()
			.with(move |world: &mut World| {
				build_code_block(world, target, &lang, &source);
				world.flush();
			})
			.await;
		Ok(())
	})
}

/// Infer the highlighter language from a file extension, treating a `.bsx` entry
/// as `jsx` (its closest highlightable grammar).
fn infer_language(src: &str) -> String {
	match src.rsplit('.').next().unwrap_or("") {
		"bsx" => "jsx".to_string(),
		ext => ext.to_string(),
	}
}

/// Build `<pre><code class="language-X">…spans…</code></pre>` under `target`,
/// highlighting `source` inline (the same span tree [`apply_syntax_highlighting`]
/// emits), so the post-parse pass need not see this async-built subtree.
fn build_code_block(world: &mut World, target: Entity, lang: &str, source: &str) {
	let spans = world.resource::<SyntaxHighlighting>().highlight(lang, source);
	let pre = world.spawn((Element::new("pre"), ChildOf(target))).id();
	let code = world.spawn((Element::new("code"), ChildOf(pre))).id();
	world.spawn((
		Attribute::new("class"),
		Value::str(format!("language-{lang}")),
		AttributeOf::new(code),
	));
	for span in spans {
		let span_el = world.spawn((Element::new("span"), ChildOf(code))).id();
		if let Some(capture) = &span.capture {
			world.spawn((
				Attribute::new("class"),
				Value::str(format!("hl-{capture}")),
				AttributeOf::new(span_el),
			));
		}
		world.spawn((Value::Str(span.text), ChildOf(span_el)));
	}
}

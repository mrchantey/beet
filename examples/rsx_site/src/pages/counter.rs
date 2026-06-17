use beet::prelude::*;

/// A reactive counter, the Rust-authored mirror of the no-code `counter.bsx`
/// page in `examples/bsx_site`.
///
/// State is a single typed document atom ([`TypedFieldRef`]); the display reads
/// it in markup (`{count}`), and each button's `PointerUp` observer mutates it
/// through [`FieldQuery`]. Document-sync fans the change back to the display
/// binding, which repaints, the same reactive loop the BSX verbs drive, with no
/// per-button mirror state.
pub fn get() -> impl Bundle {
	// `count` is a shared atom keyed in this page's document; the buttons mutate
	// it and the display reads it, both resolving the same field by key.
	let count = TypedFieldRef::<i64>::new("count");
	let more = on_count(count.clone(), 1);
	let less = on_count(count.clone(), -1);
	rsx! {
		<article>
			<h1>"Counter"</h1>
			<p>
				"A reactive counter with no custom client code: each button mutates a "
				"document field, and the count reads it back. The Rust counterpart of "
				"the no-code "<code>"counter.bsx"</code>" page."
			</p>
			// the document root the field and observers resolve against.
			<div {Document::default()} {inline_class![(common_props::MaxWidth, Length::Rem(20.))]}>
				<p>"You have clicked "{count}" times."</p>
				<div {Classes::new([crate::style::classes::DESIGN_ROW])}>
					<Button variant=(ButtonVariant::Filled) {more}>"More"</Button>
					<Button variant=(ButtonVariant::Tonal) {less}>"Less"</Button>
				</div>
			</div>
		</article>
	}
}

/// A `PointerUp` observer that adds `amount` to `count` against the activated
/// element, resolving the field through its ancestor [`Document`].
fn on_count(count: TypedFieldRef<i64>, amount: i64) -> impl Bundle {
	OnSpawn::observe(move |ev: On<PointerUp>, mut fields: FieldQuery| {
		fields
			.update_typed(ev.target, &count, |value| *value += amount)
			.ok();
	})
}

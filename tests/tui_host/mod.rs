//! Shared harness for the live-TUI acceptance suites over the on-disk
//! `examples/ui` entries: an entry and the documents it names, copied off disk
//! into an in-memory store, booted into an in-process [`ChannelTerminal`] and
//! driven with terminal bytes.
//!
//! Every piece an example is made of is unit-tested in its own crate; what only
//! a suite over the *authored* entry can show is that the pieces assemble, so
//! each suite layers its own document helpers over this host and drives the
//! real markup. The store is in-memory, seeded from the on-disk entry, so a
//! suite exercises the same load and the same write-back without rewriting the
//! example's own documents.
//!
//! Files in a `tests/` subdirectory are not compiled as their own test target,
//! so this module is `#[path]`-included by each suite, which drives the subset
//! of it the example calls for.
#![allow(dead_code)]
use beet::prelude::*;

/// The examples directory in the repo, the store every suite seeds itself
/// from.
pub const ENTRY_DIR: &str = "examples/ui";

/// A booted live-TUI app: an on-disk entry built into a router root, a page
/// host and an in-world navigator, driven through a channel terminal.
pub struct TuiHost {
	pub app: App,
	pub host: Entity,
	/// The in-memory store the entry was built from, so a suite can read back
	/// what an edit persisted, or boot a second app over it.
	pub store: BlobStore,
}

/// An SGR mouse sequence: button `button` at 0-indexed cell `(col, row)`,
/// pressed (`M`) or released (`m`).
fn sgr(button: u32, col: u32, row: u32, pressed: bool) -> Vec<u8> {
	let suffix = if pressed { 'M' } else { 'm' };
	format!("\x1b[<{button};{};{}{suffix}", col + 1, row + 1).into_bytes()
}

impl TuiHost {
	/// The files at `paths` within [`ENTRY_DIR`], copied off disk into an
	/// in-memory store.
	pub async fn seeded_store(paths: &[&str]) -> BlobStore {
		let disk = BlobStore::new(FsStore::new(
			AbsPathBuf::new_workspace_rel(ENTRY_DIR).unwrap(),
		));
		let store = BlobStore::temp();
		for path in paths {
			let path = SmolPath::from(*path);
			let bytes = disk.get(&path).await.unwrap();
			store.insert(&path, bytes).await.unwrap();
		}
		store
	}

	/// Boot the entry at `entry` within `store` at a `size`-cell viewport:
	/// build it onto a root carrying the store (so a `<DocumentBlob>` or
	/// `<SceneBlob>` resolves it by ancestry), settle the async reads, then
	/// pair a channel terminal with a page host and an in-world navigator,
	/// exactly as the binary's `--server=tui` boot does.
	pub async fn boot(store: BlobStore, entry: &str, size: UVec2) -> Self {
		let mut app = App::new();
		app.add_plugins((
			RouterPlugin,
			CharcellTuiPlugin,
			NavigatorPlugin,
			LivePagePlugin,
			material::MaterialStylePlugin,
		))
		.insert_resource(pkg_config!());

		let entry = store.get_media(&SmolPath::from(entry)).await.unwrap();
		let source = entry.as_utf8().unwrap();
		// the entry is built to be *driven*, not to boot: its `<CallOnReady>`
		// would start the real stdio `TuiServer` inside the test process, giving
		// the one `FixedPage` tree a second surface. That is not merely noisy —
		// a page displayed on two surfaces is transcluded into two layout
		// renders, and `SurfaceQuery` resolves the input's surface through the
		// *first* portal holder, so scoped input (typing) would be routed to the
		// stdio terminal rather than to this suite's channel one.
		let root = app.world_mut().spawn(DisableCallOnReady).id();
		let template =
			BsxTemplate::parse_entry(app.world_mut(), source).unwrap();
		app.world_mut()
			.entity_mut(root)
			.insert(store.clone())
			.insert_template(template)
			.unwrap();
		// the store reads that answer the documents are tasks, so the tree
		// exists frames before they do; settling here is the boot, not a fixup.
		AsyncRunner::settle_async_tasks(app.world_mut()).await;

		let router = app
			.world_mut()
			.run_system_cached_with::<_, Result<Entity>, _, _>(
				find_router,
				root,
			)
			.unwrap()
			.unwrap();
		let (channel, terminal) =
			ChannelTerminal::new(TerminalConfig::default());
		let host = app
			.world_mut()
			.spawn((
				channel,
				terminal,
				PageHost::bundle(size),
				Navigator::in_world(router, "/"),
				// deterministic frames whatever terminal runs the tests
				KittyGraphicsSupport { enabled: false },
			))
			.id();
		app.update();
		Self { app, host, store }
	}

	/// Push raw input bytes (keys, SGR mouse) into the channel terminal.
	pub fn send(&mut self, data: &[u8]) {
		self.app
			.world_mut()
			.get_mut::<ChannelTerminal>(self.host)
			.unwrap()
			.send_input(data)
			.unwrap();
	}

	/// The painted frame as rows of cells, the grid a mouse coordinate indexes.
	///
	/// A cell, not a byte: every box a generated control paints is drawn in
	/// box-drawing characters, so a byte offset into a row holding one is three
	/// times the column the terminal reports for it.
	pub fn cells(&self) -> Vec<Vec<char>> {
		self.frame()
			.lines()
			.map(|line| line.chars().collect())
			.collect()
	}

	/// The 0-indexed start cell of the `nth` (0-based) occurrence of `text`,
	/// scanning the frame top to bottom. Several generated controls share a
	/// label (every collection has an `add`), so a click names which one.
	pub fn cell_of_nth(&self, text: &str, nth: usize) -> (u32, u32) {
		let needle: Vec<char> = text.chars().collect();
		let mut seen = 0;
		for (row, line) in self.cells().into_iter().enumerate() {
			for col in 0..line.len().saturating_sub(needle.len() - 1) {
				if line[col..col + needle.len()] != needle[..] {
					continue;
				}
				if seen == nth {
					return (col as u32, row as u32);
				}
				seen += 1;
			}
		}
		panic!(
			"text {text:?} occurs fewer than {} times in frame:\n{}",
			nth + 1,
			self.frame()
		);
	}

	/// Click into the control the `nth` (0-based) occurrence of key `text`
	/// labels.
	///
	/// A generated form paints its key *above* the control it names, and an
	/// empty control has no text to aim at, so the control is located from its
	/// box rather than by an offset from the key: scan down the key's column for
	/// the box top, find its bottom, and click the middle of the interior. A
	/// click anywhere inside a text control focuses it, so the middle needs to
	/// be no more precise than "not the border".
	pub fn click_control_of(&mut self, text: &str, nth: usize) {
		let (col, row) = self.cell_of_nth(text, nth);
		let (col, row) = (col as usize, row as usize);
		let cells = self.cells();
		let at = |row: usize, col: usize| {
			cells.get(row).and_then(|line| line.get(col)).copied()
		};
		let find = |from: usize, corner: char| {
			(from..cells.len())
				.find(|&row| at(row, col) == Some(corner))
				.unwrap_or_else(|| {
					panic!(
						"no {corner:?} below occurrence {nth} of {text:?} at \
						 column {col} in frame:\n{}",
						self.frame()
					)
				})
		};
		let top = find(row + 1, '\u{250c}');
		let bottom = find(top + 1, '\u{2514}');
		let right = (col + 1..cells[top].len())
			.find(|&col| at(top, col) == Some('\u{2510}'))
			.unwrap();
		self.click(((col + right) / 2) as u32, ((top + bottom) / 2) as u32);
	}

	/// Click (press + release) the cell at `(col, row)`.
	pub fn click(&mut self, col: u32, row: u32) {
		self.send(&sgr(0, col, row, true));
		self.app.update();
		self.send(&sgr(0, col, row, false));
		self.app.update();
	}

	/// Click the `nth` occurrence of `text`.
	pub fn click_nth(&mut self, text: &str, nth: usize) {
		let (col, row) = self.cell_of_nth(text, nth);
		self.click(col, row);
	}

	/// Click the first occurrence of `text`.
	pub fn click_text(&mut self, text: &str) { self.click_nth(text, 0) }

	/// Click the last occurrence of `text`.
	pub fn click_last(&mut self, text: &str) {
		// a zero count falls through to `cell_of_nth`, whose panic names the
		// missing text and prints the frame
		let count = self.frame().matches(text).count();
		self.click_nth(text, count.saturating_sub(1));
	}

	/// Advance `frames` frames, for a settle with no distinctive needle.
	pub fn settle(&mut self, frames: usize) {
		for _ in 0..frames {
			self.app.update();
		}
	}

	/// Type `text` into the focused element, as a terminal delivers it.
	pub fn type_text(&mut self, text: &str) {
		self.send(text.as_bytes());
		self.settle(8);
	}

	/// Press Enter, activating the focused element.
	pub fn press_enter(&mut self) {
		self.send(b"\r");
		self.settle(8);
	}

	/// The painted frame as plain text.
	pub fn frame(&self) -> String {
		self.app
			.world()
			.get::<DoubleBuffer>(self.host)
			.unwrap()
			.front_buffer()
			.render_plain()
	}

	/// The painted frame with its scaled text folded back to ASCII, so a
	/// needle can name a heading's text as it was authored.
	pub fn folded_frame(&self) -> String {
		FontScale::from_fullwidth(&self.frame())
	}

	/// Advance frames until the frame contains `needle`, returning the frame.
	pub fn step_until(&mut self, needle: &str) -> String {
		self.step_until_count(needle, 1)
	}

	/// Advance frames until the frame contains `needle` at least `count` times,
	/// returning the frame. A generated widget and the draft that produced it
	/// carry the same text, so "the table grew the column" is a second
	/// occurrence rather than a first.
	pub fn step_until_count(&mut self, needle: &str, count: usize) -> String {
		self.step_until_frame(needle, count, Self::frame)
	}

	/// [`step_until`](Self::step_until) over the [`folded
	/// frame`](Self::folded_frame), for a needle in scaled text.
	pub fn step_until_folded(&mut self, needle: &str) -> String {
		self.step_until_frame(needle, 1, Self::folded_frame)
	}

	/// Advance frames until `read`'s frame contains `needle` at least `count`
	/// times, returning that frame.
	fn step_until_frame(
		&mut self,
		needle: &str,
		count: usize,
		read: fn(&Self) -> String,
	) -> String {
		for _ in 0..200 {
			self.app.update();
			let frame = read(self);
			if frame.matches(needle).count() >= count {
				return frame;
			}
		}
		panic!(
			"frame never contained '{needle}' {count} times:\n{}",
			read(self)
		);
	}

	/// The element holding keyboard focus.
	pub fn focused(&mut self) -> Entity {
		self.app
			.world_mut()
			.query_filtered::<Entity, With<Focus>>()
			.single(self.app.world())
			.expect("no focused element")
	}

	/// The document persisted at `path` in the app's own store.
	///
	/// Read through a default registry, whose intrinsic meta-schema is what a
	/// *schema* document's own declaration resolves through; a row schema a
	/// data document names is unregistered here and defers to a wildcard,
	/// which is the read this assertion wants.
	pub async fn stored(&self, path: &str) -> TypedDocument {
		let registry = SchemaRegistry::default();
		self.store
			.get_document(
				SchemaResolver::default().with_schemas(&registry),
				&SmolPath::from(path),
			)
			.await
			.unwrap()
	}
}

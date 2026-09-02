+++
title = "Extend the engine"
+++

# Extend the engine

In this tutorial we will give the guestbook a memory. By the end you will have written a compiled action, added it to the vocabulary your scene is written in, and watched the book survive a restart.

Scripts could not do this one. A script holds no authority: it cannot open a file, reach the network or touch anything the host did not hand it, which is what makes it safe to run one you did not write. Durable storage is authority, so it belongs in compiled code.

## Start from a crate

The two earlier lessons ran on the `beet` binary. This one builds its own, because the new word has to be compiled in. Make a crate:

```sh
cargo new guestbook-plugin
cd guestbook-plugin
```

Add beet and serde:

```sh
cargo add beet --features=router,http_server,fs,json,quickjs
cargo add serde --features=derive
```

`router` and `http_server` bring the servers and routes the scene already uses, `fs` the store the entry is read through, `json` the codec the book is saved in, and `quickjs` keeps the scripted routes from the last lesson running in the same binary.

## Write the action

Replace `src/main.rs` with this:

```rust
use beet::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// Where the book is kept, relative to the entry's store root.
const BOOK_PATH: &str = "guestbook.json";

/// One signature in the book.
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
struct Entry {
	name: String,
	message: String,
}

/// Append an entry to the book, then write the whole book back through the
/// nearest ancestor store.
#[action(route = "sign")]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
async fn SignGuestbook(cx: ActionContext<Entry>) -> Result<String> {
	let name = cx.input.name.clone();
	let entry = Value::from_serde(&cx.input)?;
	let book = cx
		.caller
		.with_state::<DocumentQuery, _>(move |entity, mut docs| {
			docs.with_field(entity, &book_field(), move |entries| -> Result {
				entries.as_list_mut()?.push(entry);
				Ok(())
			})??;
			docs.get(entity, &DocumentPath::Ancestor).map(Clone::clone)
		})
		.await??;
	let store = cx.caller.get_in_ancestors_cloned::<BlobStore>().await?;
	save_book(&store, &book).await?;
	Ok(format!("thanks for signing, {name}"))
}

/// The list every part of the guestbook addresses.
fn book_field() -> FieldRef {
	FieldRef::new("entries").with_init(Value::List(Vec::new()))
}

/// Write the whole book back as pretty JSON.
async fn save_book(store: &BlobStore, book: &Document) -> Result {
	let bytes = MediaType::Json
		.serialize_with_options(&book.0, SerializeOptions { pretty: true })?;
	store.insert(&SmolPath::from(BOOK_PATH), bytes).await?;
	Ok(())
}
```

Notice the last two lines of the action. It appends to the same `entries` list the page and the `list` route already read, and then reaches for a `BlobStore` in its ancestors and writes. It never names a filesystem. The store it finds is whatever the entry was loaded through, so the same action saves to a directory, to a browser's storage or to a bucket, and the code does not change.

## Register the word and load the scene

Add this below what you just wrote:

```rust
/// The vocabulary this binary adds to beet.
struct GuestbookPlugin;

impl Plugin for GuestbookPlugin {
	fn build(&self, app: &mut App) { app.register_type::<SignGuestbook>(); }
}

fn main() -> AppExit {
	App::new()
		.add_plugins(BeetPlugins)
		.add_plugins(GuestbookPlugin)
		.add_systems(Startup, load_entry)
		.run()
}

/// Build the entry on the async runtime, so every store read is awaited.
fn load_entry(world: &mut World) {
	world.run_async_local(async move |world: AsyncWorld| {
		if let Err(err) = build_entry(&world).await {
			error!("{err}");
			world.write_message(AppExit::error()).await;
		}
	});
}

/// Read `main.bsx` and the saved book through the store, then build the scene
/// onto a root carrying both.
async fn build_entry(world: &AsyncWorld) -> Result {
	let store = BlobStore::new(FsStore::new(AbsPathBuf::new(".")?));
	let source = store
		.get_media(&SmolPath::from("main.bsx"))
		.await?
		.as_utf8()?
		.to_string();
	let book = load_book(&store).await?;
	world
		.with(move |world: &mut World| -> Result {
			let template = BsxTemplate::parse_entry(world, &source)?;
			let root = world.spawn((store, book)).id();
			world.entity_mut(root).insert_template(template)?;
			Ok(())
		})
		.await
}

/// The saved book, empty when nothing was ever signed.
async fn load_book(store: &BlobStore) -> Result<Document> {
	let path = SmolPath::from(BOOK_PATH);
	match store.exists(&path).await? {
		true => store
			.get(&path)
			.await?
			.xmap(|bytes| MediaType::Json.deserialize::<Value>(&bytes))?
			.xmap(Document::new)
			.xok(),
		false => Document::new(value!({ "entries": [] })).xok(),
	}
}
```

`main` is the whole story of this lesson: beet's plugins, then yours, then the scene. `BeetPlugins` is the same library of capabilities the `beet` binary links; `GuestbookPlugin` adds one more; and `load_entry` reads `main.bsx` and builds it, exactly as the `beet` binary does, except that this binary also loads the book first.

Remember that the book is loaded *before* the scene builds. The scene's `<CallOnReady>` starts its servers the instant the tree lands, so a request could arrive before a later load had finished.

## Swap one line of the scene

Copy your `main.bsx` from the previous lesson into this crate, next to `Cargo.toml`, and change the sign route to your new action:

```jsx
<CallOnReady {(CliServer, HttpServer{port:8080})}>
<Router>
	<SignGuestbook/>
	<FieldRoute path="list" field="entries" verb="Read" list/>
	<Route path="/" {FixedPage}>
		<h1>Guestbook</h1>
		<ul bx:for="entries" bx:key="name">
			<li>{@doc:name} says {@doc:message}</li>
		</ul>
	</Route>
</Router>
</CallOnReady>
```

That is the whole change to the scene: one tag. `#[action(route = "sign")]` gave the action its path along with its handler, so `<SignGuestbook/>` needs no props.

## Sign it, and restart it

```sh
cargo run -- --server=cli sign --body='{"name":"Ada","message":"hello"}'
```

```text
INFO Get /sign -> 200 OK in 33 ms
"thanks for signing, Ada"
```

Look in the directory:

```sh
cat guestbook.json
```

```text
{
  "entries": [
    {
      "message": "hello",
      "name": "Ada"
    }
  ]
}
```

Sign it once more, then read the book back in a completely fresh process:

```sh
cargo run -- --server=cli sign --body='{"name":"Grace","message":"first computer bug"}'
cargo run -- --server=cli list
```

```text
INFO Get /list -> 200 OK in 1 ms
[{"message":"hello","name":"Ada"},{"message":"first computer bug","name":"Grace"}]
```

Notice that there is no server running. Each of those commands was a separate process that started, read the book off disk, answered and exited. In the first lesson the same three commands gave you an empty list every time. The book remembers now.

## Move the route without recompiling

One last thing, to see what did not change. Open `main.bsx` and replace `<SignGuestbook/>` with this:

```jsx
	<Route path="guests/sign" {SignGuestbook}/>
```

Run it again without rebuilding anything:

```sh
cargo run -- --server=cli guests sign --body='{"name":"Alan","message":"hi"}'
```

```text
INFO Get /guests/sign -> 200 OK in 33 ms
"thanks for signing, Alan"
```

The compiled action still answers to the scene. Where it lives in the url space was never its decision.

## What you have built

You have extended the engine. The guestbook now has a compiled action beside its scripted ones and its authored markup, and all three are addressed the same way, from the same file, by the same reader.

Look back at what stayed still. The guestbook you have at the end of these three lessons is the guestbook you started with: one scene, describing a tool. You shaped it by hand, you taught it new words with scripts, and you extended it with compiled Rust, and at no point did you throw the previous version away and start again. That is the slope these three layers exist to make gentle.

The [Scene Format](/docs/scene-format) page is where that idea goes next: the file you have been editing is a draft of a standard, and it is meant to outgrow beet.

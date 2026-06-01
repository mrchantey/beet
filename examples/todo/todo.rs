//! # Todo - a CRUD CLI backed by a document
//!
//! A command-line todo app built on the same routing stack as an HTTP
//! server: each subcommand is a route and the arguments map to a
//! [`Request`]. The `--body` argument is lifted into the request body,
//! exactly as a JSON payload would be on an HTTP `POST`.
//!
//! The todo list lives in the `todos` field of a [`Document`], which is
//! loaded from `examples/todo/todos.json` and rewritten whenever a command
//! mutates it.
//!
//! ## Running the Example
//!
//! ```sh
//! # append a todo
//! cargo run --example todo --features=router,json -- create --body='{"description":"take out garbage","done":false}'
//!
//! # list every todo
//! cargo run --example todo --features=router,json -- list
//!
//! # read a single todo by its index
//! cargo run --example todo --features=router,json -- read --body=0
//!
//! # replace the todo at an index
//! cargo run --example todo --features=router,json -- update --body='[0,{"description":"take out garbage","done":true}]'
//!
//! # remove the todo at an index
//! cargo run --example todo --features=router,json -- delete --body=0
//! ```
use beet::prelude::*;
use serde::Deserialize;
use serde::Serialize;

const TODOS_FILE: &str = "examples/todo/todos.json";

/// A single todo item.
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
struct Todo {
	description: String,
	done: bool,
}

#[beet::main]
async fn main() -> Result {
	// DocumentPlugin drives the bidi sync the self-bound actions rely on: each
	// route's local `Value` mirrors the shared `todos` field
	let mut world = (AsyncPlugin, RouterPlugin, DocumentPlugin).into_world();

	// load the persisted list, falling back to an empty document
	let document = load_document().await?;
	let before = document.0.clone();

	let root = world
		.spawn((document, router(), children![
			create(),
			read(),
			update(),
			delete(),
			list(),
		]))
		.flush();

	// settle the read path so each route's `Value` mirrors the loaded list
	// before the action runs (the action body executes at dispatch, ahead of
	// the call's own update loop)
	world.update_local();

	// a CLI invocation is just a request: the path selects the route and
	// `--body` becomes the request body
	let request = Request::from_cli_args(CliArgs::parse_env());
	let response = world
		.entity_mut(root)
		.call::<Request, Response>(request)
		.await?;

	cross_log!("{}", response.pretty_text().await?);

	// persist only when a command actually changed the list
	let after = world.entity(root).get::<Document>().unwrap().0.clone();
	if after != before {
		save_document(&after).await?;
	}
	Ok(())
}

// ╔═══════════════════════════════════════════╗
// ║   Routes                                  ║
// ╚═══════════════════════════════════════════╝
//
// Each route binds the shared `todos` field to an action, reusing the
// generic document actions where they fit and providing bespoke ones for
// index-addressed reads and updates.

/// The list field that every route operates on.
fn todos() -> FieldRef {
	FieldRef::new("todos").with_init(Value::List(Vec::new()))
}

/// `create --body='{..}'` — append a todo to the list.
fn create() -> impl Bundle {
	(
		todos(),
		exchange_route("create", PushField::<Todo>::default()),
	)
}

/// `read --body=<index>` — return a single todo by its index.
fn read() -> impl Bundle { (todos(), exchange_route("read", ReadTodo)) }

/// `update --body='[<index>,{..}]'` — replace the todo at an index.
fn update() -> impl Bundle { (todos(), exchange_route("update", UpdateTodo)) }

/// `delete --body=<index>` — remove the todo at an index.
fn delete() -> impl Bundle {
	(todos(), exchange_route("delete", RemoveAtField))
}

/// `list` — return the entire list.
fn list() -> impl Bundle { (todos(), exchange_route("list", ReadField)) }

/// Returns a single todo by its index, erroring when out of bounds.
///
/// Self-bound: reads the entity's own [`Value`], which bidi sync mirrors from
/// the `todos` field.
#[action]
#[derive(Default, Clone, Component)]
fn ReadTodo(
	cx: In<ActionContext<usize>>,
	values: Query<&Value>,
) -> Result<Value> {
	let index = cx.input;
	values
		.get(cx.id())?
		.as_list()?
		.get(index)
		.cloned()
		.ok_or_else(|| bevyhow!("no todo at index {index}"))
}

/// Replaces the todo at the given index, erroring when out of bounds.
///
/// Self-bound: mutates the entity's own [`Value`]; write-back reaches the
/// document.
#[action]
#[derive(Default, Clone, Component)]
fn UpdateTodo(
	cx: In<ActionContext<(usize, Todo)>>,
	mut values: Query<&mut Value>,
) -> Result {
	let entity = cx.id();
	let (index, todo) = cx.take();
	let todo = Value::from_serde(&todo)?;
	*values
		.get_mut(entity)?
		.as_list_mut()?
		.get_mut(index)
		.ok_or_else(|| bevyhow!("no todo at index {index}"))? = todo;
	Ok(())
}

// ╔═══════════════════════════════════════════╗
// ║   Persistence                             ║
// ╚═══════════════════════════════════════════╝

/// Loads the document from [`TODOS_FILE`], starting fresh when absent.
async fn load_document() -> Result<Document> {
	match fs_ext::read_to_string_async(TODOS_FILE).await {
		Ok(json) => MediaType::Json
			.deserialize::<Value>(json.as_bytes())
			.map(Document::new),
		Err(_) => Document::new(val!({ "todos": [] })).xok(),
	}
}

/// Writes the document back to [`TODOS_FILE`] as pretty JSON.
async fn save_document(value: &Value) -> Result {
	let bytes = MediaType::Json
		.serialize_with_options(value, SerializeOptions { pretty: true })?;
	fs_ext::write_async(TODOS_FILE, bytes).await?;
	Ok(())
}

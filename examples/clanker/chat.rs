//! # Clanker Chat
//!
//! An example of a chat CLI
//!
//! If the clanker read the tool call it should mention the hidden number 777.
//! ```sh
//! cargo run --example chat --features=clanker,native-tls whats 1+1. use the tool.
//!	```
//!
//! Note that I get about a 50/50 success that it read the tool call.
//!
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins(MinimalPlugins)
		.init_plugin::<ClankerPlugin>()
		.add_systems(Startup, create_scene)
		// .add_systems(PostStartup, run_clanker)
		// .add_observer(exit_on_complete)
		.run();
}



fn create_scene(mut commands: Commands, mut query: ContextQuery) -> Result {
	// 1. define actors
	let clanker_id = query.actors_mut().insert(Actor::clanker());
	let system_id = query.actors_mut().insert(Actor::system());
	let user_id = query.actors_mut().insert(Actor::user());

	// clanker thread is the thread sent to the model
	let clanker_thread = query.threads_mut().insert(
		Thread::default().with_actors([system_id, clanker_id, user_id]),
	);

	// user thread is the thread printed to stdout
	let user_thread = query
		.threads_mut()
		.insert(Thread::display_only().with_actors([clanker_id, user_id]));

	// 2. define relations
	commands
		.spawn((system_id, Sequence::new(), Repeat, children![
			(
				clanker_id,
				clanker_thread,
				ModelAction::new(OllamaProvider::default()).streaming()
			),
			(
				user_id,
				user_thread,
				stdin.into_tool(),
				StdoutCursor::default(),
				OnSpawn::observe(log_name),
				OnSpawn::observe(listen_for_changes)
			)
		]))
		.call::<(), Outcome>((), default());

	// 3. define items
	query.add_items(Item::new(
		system_id,
		ItemStatus::Completed,
		"you are robot, make beep boop noises",
	))?;
	Ok(())
}

#[tool]
fn stdin(
	entity: SystemToolIn,
	mut query: ContextQuery,
	actors: Query<&ActorId>,
) -> Result<Outcome> {
	let owner = actors.get(entity.caller)?;
	let mut input = String::new();
	print!("\nUser > ");
	std::io::Write::flush(&mut std::io::stdout())?;
	std::io::stdin().read_line(&mut input)?;
	query.add_items(Item::new(*owner, ItemStatus::Completed, input))?;
	Ok(Pass(()))
}

#[allow(unused)]
#[derive(Default, Component)]
struct StdoutCursor(HashMap<ItemId, u32>);

fn log_name(ev: On<EntityItemCreated>, context_query: ContextQuery) -> Result {
	let item = context_query.items().get(ev.item)?;
	let actor = context_query.actors().get(item.owner())?;
	println!("<< {} >> ", actor.name());
	Ok(())
}

fn listen_for_changes(
	ev: On<EntityItemUpdated>,
	context_query: ContextQuery,
	mut query: Query<&mut StdoutCursor>,
) -> Result {
	let mut cursor = query.get_mut(ev.entity)?;
	let item = context_query.items().get(ev.item)?;
	let content = item.content().to_string();
	let cursor_item = cursor.0.entry(ev.item).or_insert(0);

	let new_content = &content[*cursor_item as usize..];
	print!("{}", new_content);
	*cursor_item = content.len() as u32;

	Ok(())
}

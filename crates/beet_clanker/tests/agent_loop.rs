use beet_clanker::prelude::*;
use beet_core::prelude::*;


#[beet_core::test(timeout_ms = 15_000)]
async fn main() {
	let mut world = World::new();
	let mut context = Context::new();

	let system = context.new_actor(ActorKind::System);
}

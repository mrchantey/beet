use crate::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;




#[tool]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn SkipIfLastest(
	cx: AsyncToolIn<((), Next<(), Outcome>)>,
) -> Result<Outcome> {
	let should_skip = cx
		.caller
		.with_state::<ThreadQuery, _>(|entity, query| -> Result<bool> {
			if let Some(last) = query.thread(entity)?.posts().into_iter().last()
				&& last.actor_entity == entity
			{
				// i was the latest actor to post, should be quiet for a bit
				true
			} else {
				false
			}
			.xok()
		})
		.await?;

	if should_skip {
		Ok(PASS)
	} else {
		cx.1.call(()).await
	}
}

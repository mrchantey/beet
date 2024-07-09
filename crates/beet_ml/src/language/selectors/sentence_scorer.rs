use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use forky_core::ResultTEExt;
use std::borrow::Cow;

/// This component is for use with the [`SentenceScorer`]. Add to either the agent or a child behavior.
#[derive(Debug, Clone, Component, PartialEq, Reflect)]
#[reflect(Component)]
pub struct Sentence(pub Cow<'static, str>);
impl Sentence {
	pub fn new(s: impl Into<Cow<'static, str>>) -> Self { Self(s.into()) }
}

/// Updates the [`Score`] of each child based on the similarity of its [`Sentence`] with the agent,
/// for use with [`ScoreSelector`]
#[derive(Debug, Default, Clone, PartialEq, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[category(ActionCategory::ChildBehaviors)]
#[systems(sentence_scorer.in_set(TickSet))]
pub struct SentenceScorer;

impl SentenceScorer {
	pub fn new() -> Self { Self {} }
}

fn sentence_scorer(
	mut commands: Commands,
	mut berts: ResMut<Assets<Bert>>,
	sentences: Query<&Sentence>,
	// TODO double query, ie added running and added asset
	started: Query<
		(&SentenceScorer, &Handle<Bert>, &TargetAgent, &Children),
		Added<Running>,
	>,
) {
	for (_scorer, handle, agent, children) in started.iter() {
		let Some(bert) = berts.get_mut(handle) else {
			continue;
		};

		let children = children.into_iter().cloned().collect::<Vec<_>>();
		//todo: async
		bert.score_sentences(agent.0, children, &sentences)
			.ok_or(|e| log::error!("{e}"))
			.map(|scores| {
				for (entity, _, score) in scores {
					commands.entity(entity).insert(Score::Weight(score));
				}
			});
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use beet_ecs::prelude::*;
	use bevy::prelude::*;
	use sweet::*;

	fn setup(app: &mut App) -> Entity {
		let handle = app
			.world_mut()
			.resource_mut::<AssetServer>()
			.load::<Bert>("default-bert.ron");

		app.world_mut()
			.spawn(Sentence::new("destroy"))
			.with_children(|parent| {
				let id = parent.parent_entity();
				parent
					.spawn((
						TargetAgent(id),
						handle,
						SentenceScorer::default(),
						ScoreSelector {
							consume_scores: true,
						},
						Running,
					))
					.with_children(|parent| {
						parent.spawn(Sentence::new("heal"));
						parent.spawn(Sentence::new("kill"));
					});
			})
			.id()
	}


	#[test]
	fn works() -> Result<()> {
		pretty_env_logger::try_init().ok();

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			BertPlugin::default(),
			LifecyclePlugin,
		))
		.finish();

		block_on_asset_load::<Bert>(&mut app, "default-bert.ron");
		let entity = setup(&mut app);
		app.update();

		let tree = EntityTree::new_with_world(entity, app.world());

		let _scorer = app
			.world_mut()
			.query::<&SentenceScorer>()
			.iter(app.world())
			.next()
			.unwrap();

		let scores = tree.component_tree::<Score>(app.world());

		let heal_score = scores.children[0].children[0].value.unwrap();
		let kill_score = scores.children[0].children[1].value.unwrap();
		expect(kill_score).to_be_greater_than(heal_score)?;
		expect(heal_score.weight()).to_be_less_than(0.5)?;
		expect(kill_score.weight()).to_be_greater_than(0.5)?;

		expect(tree.component_tree(app.world())).to_be(
			Tree::new(None).with_child(
				Tree::new(Some(&Running)).with_leaf(None).with_leaf(None),
			),
		)?;
		app.update();
		expect(tree.component_tree(app.world())).to_be(
			Tree::new(None).with_child(
				Tree::new(Some(&Running))
					.with_leaf(None)
					.with_leaf(Some(&Running)),
			),
		)?;

		Ok(())
	}
}

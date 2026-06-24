//! The data form of the `hello_ml` sentence-similarity demo: a markup agent that,
//! once its [`Bert`] model loads, logs the child [`Sentence`] closest to its prompt.
use crate::beet::prelude::*;
use beet_core::prelude::*;

/// A sentence-matching agent: a prompt [`Sentence`] plus a [`NearestSentence`] over a
/// [`Bert`] model, eg `<NearestSentenceAgent prompt="please kill the baddies"
/// src="ml/default-bert.ron"><div {Sentence("heal")}/><div {Sentence("attack")}/>
/// </NearestSentenceAgent>`. The candidate sentences are children (via `<Slot/>`);
/// `Handle<Bert>` is minted from the [`AssetServer`] here because a handle is not a
/// markup value. [`choose_nearest_on_load`] logs the winner once the model loads.
#[template(system)]
pub fn NearestSentenceAgent(
	#[prop(into)] prompt: String,
	#[prop(into)] src: String,
	asset_server: Res<AssetServer>,
) -> impl Bundle {
	rsx! {
		<span {(
			Sentence::new(prompt),
			NearestSentence::new(asset_server.load(src)),
			ChooseNearestOnLoad,
		)}><Slot/></span>
	}
}

/// A candidate sentence child for a [`NearestSentenceAgent`], eg
/// `<SentenceOption text="attack"/>`. Wraps [`Sentence`] so the text rides a string
/// markup prop, since `Sentence`'s `Cow<'static, str>` field does not coerce from a
/// bare markup spread value.
#[template]
pub fn SentenceOption(#[prop(into)] text: String) -> impl Bundle {
	Sentence::new(text)
}

/// The user-driven variant of [`NearestSentenceAgent`], the data form of the
/// `hello_ml_chat` demo: instead of matching once on load, it re-matches every
/// time the user submits a sentence in the terminal. A [`TriggerWithUserSentence`]
/// overwrites the agent's prompt [`Sentence`] with the typed text and calls
/// [`chat_nearest_sentence`], which logs the closest child [`Sentence`] back to the
/// terminal. Candidate sentences are children (via `<Slot/>`); `Handle<Bert>` is
/// minted from the [`AssetServer`] here because a handle is not a markup value.
#[template(system)]
pub fn ChatSentenceAgent(
	#[prop(into)] src: String,
	asset_server: Res<AssetServer>,
) -> impl Bundle {
	rsx! {
		<span {(
			TriggerWithUserSentence,
			BertHandle(asset_server.load(src)),
			Action::<(), Outcome>::new_system(chat_nearest_sentence),
		)}><Slot/></span>
	}
}

/// The [`Bert`] handle for a [`ChatSentenceAgent`], wrapped so the asset survives
/// on the agent (the [`Action`] reads it on each user sentence). [`NearestSentence`]
/// cannot be reused here: it requires an `Action<(), Entity>`, but
/// [`TriggerWithUserSentence`] calls an `Action<(), Outcome>`.
#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct BertHandle(pub Handle<Bert>);

/// Action: on each user sentence, match the agent's prompt [`Sentence`] against its
/// descendant candidate [`Sentence`]s and log the winner to the terminal. The
/// runtime form of `hello_ml_chat`'s chooser; soft-fails while the [`Bert`] model
/// streams in so an early keypress does not panic the app.
fn chat_nearest_sentence(
	cx: In<ActionContext>,
	mut berts: ResMut<Assets<Bert>>,
	mut log: MessageWriter<OnLogMessage>,
	children: Query<&Children>,
	sentences: Query<&Sentence>,
	query: Query<(&Sentence, &BertHandle)>,
) -> Result<Outcome> {
	let agent = cx.caller.id();
	let (prompt, bert) = query.get(agent)?;
	let Some(mut bert) = berts.get_mut(&bert.0) else {
		warn!("Bert asset not yet loaded, ignoring user sentence");
		return Ok(Outcome::FAIL);
	};
	// the `<Slot/>` nests the candidate sentences below the agent, so search all
	// descendants (not just direct children) for those carrying a `Sentence`.
	let candidates = children
		.iter_descendants(agent)
		.filter(|entity| sentences.contains(*entity))
		.collect::<Vec<_>>();
	if candidates.is_empty() {
		bevybail!("ChatSentenceAgent: no candidate sentences to choose from");
	}
	let chosen =
		bert.closest_sentence_entity(prompt.0.clone(), candidates, &sentences)?;
	let text = sentences
		.get(chosen)
		.map(|sentence| sentence.0.to_string())
		.unwrap_or_default();
	log.write(
		OnLogMessage::new(format!("Agent: {text}"))
			.with_color(OnLogMessage::GAME_COLOR.into())
			.and_log(),
	);
	Ok(Outcome::PASS)
}

/// Marks a [`NearestSentenceAgent`] to log its nearest child [`Sentence`] once its
/// [`Bert`] model loads, then clears itself (a one-shot, like `hello_ml`'s `Pending`).
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub struct ChooseNearestOnLoad;

/// Once the agent's [`Bert`] finishes loading, match the prompt against the child
/// [`Sentence`]s and log the winner. The runtime form of `hello_ml`'s chooser system,
/// so the inference actually runs (on the GPU shared with Bevy when `wgpu` is the
/// burn backend).
pub(crate) fn choose_nearest_on_load(
	mut commands: Commands,
	mut berts: ResMut<Assets<Bert>>,
	mut exit: MessageWriter<AppExit>,
	children: Query<&Children>,
	sentences: Query<&Sentence>,
	query: Query<
		(Entity, &Sentence, &NearestSentence),
		With<ChooseNearestOnLoad>,
	>,
) -> Result {
	for (entity, prompt, near) in query.iter() {
		// the model streams in over a few frames, so wait rather than error.
		let Some(mut bert) = berts.get_mut(&near.bert) else {
			continue;
		};
		// the `<Slot/>` nests the candidate sentences below the agent, so search all
		// descendants (not just direct children) for those carrying a `Sentence`.
		let candidates = children
			.iter_descendants(entity)
			.filter(|entity| sentences.contains(*entity))
			.collect::<Vec<_>>();
		if candidates.is_empty() {
			continue;
		}
		let chosen = bert.closest_sentence_entity(
			prompt.0.clone(),
			candidates,
			&sentences,
		)?;
		let text = sentences
			.get(chosen)
			.map(|sentence| sentence.0.to_string())
			.unwrap_or_default();
		info!("NearestSentence chose: \"{text}\" for \"{}\"", prompt.0);
		commands.entity(entity).remove::<ChooseNearestOnLoad>();
		// the demo runs one inference; exit cleanly once it has logged.
		exit.write(AppExit::Success);
	}
	Ok(())
}

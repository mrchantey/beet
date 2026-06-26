//! The data form of the `hello_ml` sentence-similarity demo: a markup agent that,
//! once its [`Bert`] model loads, logs the child [`Sentence`] closest to its prompt.
use crate::beet::prelude::*;
use beet_core::prelude::*;

/// A sentence-matching agent: a prompt [`Sentence`] plus a [`NearestSentence`] over a
/// [`Bert`] model, eg `<NearestSentenceAgent prompt="please kill the baddies"
/// src="ml/default-bert.ron"><SentenceOption text="heal"/><SentenceOption text="attack"/>
/// </NearestSentenceAgent>`. The candidate sentences are children (via `<Slot/>`);
/// `Handle<Bert>` is minted through the deferred [`BuildAssets`] path because a handle
/// is not a markup value, so `LoadTemplate` waits for the model. [`RunOnLoad`] then
/// fires [`choose_nearest`] once, which logs the winning [`Sentence`] and exits.
#[template(system)]
pub fn NearestSentenceAgent(
	#[prop(into)] prompt: String,
	#[prop(into)] src: String,
	mut assets: BuildAssets,
) -> impl Bundle {
	rsx! {
		<span {(
			Sentence::new(prompt),
			NearestSentence::new(assets.load::<Bert>(src)),
			RunOnLoad,
			Action::<(), Outcome>::new_system(choose_nearest),
		)}><Slot/></span>
	}
}

/// A candidate sentence child for a [`NearestSentenceAgent`], eg
/// `<SentenceOption text="attack"/>`. Wraps [`Sentence`] so the text rides a string
/// markup prop, since `Sentence`'s `Cow<'static, str>` field does not coerce from a
/// bare markup spread value (a `{Sentence("attack")}` spread reflects the string as a
/// `String`, which `Cow<'static, str>` cannot accept via `from_reflect`).
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
/// minted through the deferred [`BuildAssets`] path because a handle is not a markup
/// value.
#[template(system)]
pub fn ChatSentenceAgent(
	#[prop(into)] src: String,
	mut assets: BuildAssets,
) -> impl Bundle {
	rsx! {
		<span {(
			TriggerWithUserSentence,
			BertHandle(assets.load::<Bert>(src)),
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
/// streams in so an early keypress does not panic the app. The model is loaded
/// eagerly (not deferred-gated like the load path) because the action is
/// user-triggered, so the soft-fail covers a keypress before the asset settles.
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
	let text = closest_descendant_sentence(
		agent, &prompt.0, &mut bert, &children, &sentences,
	)?;
	log.write(
		OnLogMessage::new(format!("Agent: {text}"))
			.with_color(OnLogMessage::GAME_COLOR.into())
			.and_log(),
	);
	Ok(Outcome::PASS)
}

/// Action: once the agent's [`Bert`] finishes loading (via [`RunOnLoad`], so the
/// deferred-load path guarantees the model is ready), match the prompt against the
/// child [`Sentence`]s, log the winner, and exit. The runtime form of `hello_ml`'s
/// chooser, replacing the per-frame `choose_nearest_on_load` poll: a one-shot run on
/// `LoadTemplate` rather than a loop that waited for `berts.get_mut` each frame.
fn choose_nearest(
	cx: In<ActionContext>,
	mut berts: ResMut<Assets<Bert>>,
	mut exit: MessageWriter<AppExit>,
	children: Query<&Children>,
	sentences: Query<&Sentence>,
	query: Query<(&Sentence, &NearestSentence)>,
) -> Result<Outcome> {
	let agent = cx.caller.id();
	let (prompt, near) = query.get(agent)?;
	// the deferred load guarantees the asset is ready by the time `RunOnLoad` fires.
	let mut bert = berts
		.get_mut(&near.bert)
		.ok_or_else(|| bevyhow!("Bert asset not loaded on RunOnLoad"))?;
	let text = closest_descendant_sentence(
		agent, &prompt.0, &mut bert, &children, &sentences,
	)?;
	info!("NearestSentence chose: \"{text}\" for \"{}\"", prompt.0);
	// the demo runs one inference; exit cleanly once it has logged.
	exit.write(AppExit::Success);
	Ok(Outcome::PASS)
}

/// Match `prompt` against the candidate [`Sentence`]s in `agent`'s descendants and
/// return the winner's text. The `<Slot/>` nests the candidates below the agent, so
/// the search walks all descendants (not just direct children). Shared by the load
/// ([`choose_nearest`]) and chat ([`chat_nearest_sentence`]) choosers.
fn closest_descendant_sentence(
	agent: Entity,
	prompt: &str,
	bert: &mut Bert,
	children: &Query<&Children>,
	sentences: &Query<&Sentence>,
) -> Result<String> {
	let candidates = children
		.iter_descendants(agent)
		.filter(|entity| sentences.contains(*entity))
		.collect::<Vec<_>>();
	if candidates.is_empty() {
		bevybail!("no candidate sentences to choose from");
	}
	let chosen = bert.closest_sentence_entity(
		prompt.to_string(),
		candidates,
		sentences,
	)?;
	sentences
		.get(chosen)
		.map(|sentence| sentence.0.to_string())
		.unwrap_or_default()
		.xok()
}

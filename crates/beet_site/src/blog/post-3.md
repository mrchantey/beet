+++
title="The Full Moon Harvest #3"
created="2025-09-06"
+++

# The Full Moon Harvest #3

Bevy's Five and Beet's Alive!

<iframe width="941" height="538" src="https://www.youtube.com/embed/J0zxMdsAk_I" title="Full Moon Harvest #3 | Big Diff" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

<br/>
<br/>


It's been a very good year, but for this reflection I'd like to share my journey of seemingly relentless deviations from The Way of the ECS.

## Simple Is Hard

The ECS approach to structuring data is deceptively simple, all instances of a concept are entities and all associated data lies in itty-bitty components. That is all. I've known this for many years, so why did it take me three ground-up rewrites of `beet_flow` to discover behavior trees are best represented as plain ol' entity hierarchies? Why did I spend three months writing `beet_rsx` as a bowl of enum spaghetti held together by bubble gum, only to discover that dom trees are best represented as plain ol'.. you get the picture.

It seems I'm not alone here, in particular many of the game AI crates I've poked around share this same issue, introducing new paradigms, new macros, new complexity that cuts against the grain of ECS (shoutout to [bevy_gearbox](https://crates.io/crates/bevy_gearbox) for not doing this!). I'm not sure whether this is a habit we've built up working with less versatile paradigms, or if messy architecture is a nessecary step in the process of finding the best fit.
When Tim Sweeney was recently asked what he admired most about John Carmack he had [this to say](https://lexfridman.com/tim-sweeney-transcript/#chapter12_john_carmack) about Carmack's computer graphics breakthroughs:
> They were like his seventh or eighth try after he’d done something time and time again, tried it, found a better approach, thrown out the old one, built it again, and continually rewrite his code until he found the absolute best solution to a problem. I think that stands as a lesson for every programmer to pick up on.

It reminds me a bit of bevy's fearless approach to redesigning both internals and public APIs.

## Learning The Way

Either way I like to think I'm starting to learn my lesson, for example recently working with LLM apis there's a lot of temptations:
- Ooh maybe we should build an `LlmMessage` abstraction that covers features from all the models..
- Wait but this model doesnt have that feature so we should probably create some interlinked trait system..
- Hmmm they're also stateful so what if we had like an `LlmSession` hashmap to store previous messages..

And with each abstraction the merry-go-round of complexity starts again.

I even found myself pushing back against the idea of writing it ECS-first: Well even if we wanted to use ECS you cant cos its all async.. I think this is where ECS-as-a-dicipline comes in, better to spend three days sorting out a sensible async ECS pattern, than to put on your architecture astronaut helment and spend two months lost in space (something ive done more times than I'm proud of). And yes, I did spend the first few hours building up horrific abstractions before catching myself in the act.

### Example
This snippet can be adapted for any llm provider. We communicate directly between a stateful api and the `World` via simple `CommandQueue` channels.

No traits, no enums, no macros.

```rust
fn handle_input(
  trigger: On<UserInput>,
  query: Query<&OllamaProvider>,
  mut commands: Commands,
) -> Result {
  let request = query.get(trigger.target())?.completions_request(&input)?;
  let text_entity = commands.spawn(TextContent::default()).id();
  // spawn an async task
  commands.run_system_cached_with(AsyncTask::spawn,
    async move || {
      let mut stream = request.send().await?.event_source().await?;
      while let Some(ev) = stream.next().await {
        // on each entry, queue an append command
        queue.entity(*text_entity).get_mut::<TextContent>(
          move |mut content| {
            content.push_str(&new_text);
            println!("content: {}", **content)
          }
        );
      }
      Ok(())
    }
  )
}
```

## Big Diff

In Beet news we have the beginnings of DOM diffing, the starter app is now a `todo-app` supporting insertion and removal of dom nodes. It has some glaring bugs that I have an idea for how to fix but am deciding not to until I can prove they are fixed. Thats where in-browser DOM testing the likes of cypress and playwright comes in, and I'm looking forward to sewing the seeds for that in the new moon.

This is the warmest online community I've been a part of and I'm so excited to be on this epic adventure with my bevy fam, Happy Birthday Bevy!

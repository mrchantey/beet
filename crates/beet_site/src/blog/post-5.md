+++
title="The Full Moon Harvest #5"
created="2025-11-05"
+++

# The Full Moon Harvest #5

Declarative State

<iframe width="941" height="538" src="https://www.youtube.com/embed/BhLvfvw1rgw" title="Full Moon Harvest #5 | Declarative State" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

<br/>
<br/>

Writing a metaframework can be surprisingly straightforward: Start with an unmet need, and copy-pasta the best modern patterns and principles that align with this need. So far the web layer of beet has been entirely based on Astro which is taking the web dev world by storm with a single principle:

> Make writing performant web apps a delightful developer experience.

But porting Astro's islands architecture from JS to rust has surfaced some major drawbacks:
1. Hybrid server-client architecture means two recompilations per change, doubling the achilles heel of rust DX.
2. Wasm binary sizes are quite large, big enough to present a noticable hydration lag.

The creators of `dioxus` and `leptos` are working hard to overcome these issues with projects like `subsecond` and wasm bundle splitting but I feel like we're fixing symptoms of a deeper underlying issue, so for the past month I've taken a step back and started researching alternative approaches from first principles.

### State Binding

HTML provides declarative rendering, but when it comes to binding state to the DOM we traditionally need to reach for imperative binding patterns:

```rust
// app.rs
fn Counter() -> impl Template {
  const (count, set_count) = signal(0);
  rsx!{
    <button onclick={|| set_count(count() + 1)}>
      Clicked {count} Times
    </button>
  }
}
```

Imperative state binding in general is error-prone, all the best engineering practices in the world couldn't save Cloudflare [from DDoSing itsself](https://blog.cloudflare.com/deep-dive-into-cloudflares-sept-12-dashboard-and-api-outage/) two months ago due to a classic `useEffect` bug we can all relate to, I clearly remember my boss helping me debug a pesky spinner `useEffect` I wrote that was causing a 60fps full page rerender.

HTMX circumnavigates this by pairing declarative template directives with SSR, demonstrating a crucial insight about web development:

> Most web apps are CRUD apps and most CRUD apps don't need custom client-side code

```rust
// index.html
<button hx-post="/clicked" hx-swap="innerHTML">Clicked 0 Times</button>
// server.rs
Router::new().route("/clicked", |mut state: State<u32>| {
  state += 1;
  format!("Clicked {state} Times")
});
```

Now we have a gloriously thin client and the surface-area for bugs has largely been constrained to the server, but it does com at a cost. We've broken colocation as our counter is now spread across two files, and we've introduced a 200ms server trip *per every interaction*.

Interesting but lets keep looking.

### State Synchronization

Sync Engines do a similar thing for state synchronization to what HTMX does for state DOM binding:

```jsx
// client.jsx
function App() {
  const [count] = useQuery(zero.query.counter.get());

  const increment = () => {
    zero.mutate.counter.update(count + 1);
  };

  return <button onClick={increment}>Clicked {count} Times</button>
}
```

This solves the 200ms roundtrip problem but we're back to imperative DOM binding, something we're trying to avoid.

### Synchronized State Binding

What if we combined these patterns, could we get the best of both worlds? This has been my focus of research in the last few weeks, the idea is for the framework to generate a declarative manifest of all state and templates, and for a **pre-compiled** client library to use the manifest for all state and rendering operations:

```rust
// client.rs
fn Counter()-> impl Template {
  let count = State::new("count", 0);
  rsx!{
 	  <button bx:click={count.increment}>Clicked {count} Times</button>
  }
}
```

At a glance this looks almost identical to your garden variety `solidjs` component, but now lets see the html this compiles to:

```html
// index.html
<button id="counter" type="button" data-state-id="0">Clicked 0 Times</button>
<script data-state-manifest type="application/json">
{
"directives": [
  {
    "kind": "handle_event",
    "el_state_id": 0,
    "field_path": "count",
    "event": "click",
    "action": "increment",
  },
  {
    "kind": "render_text_content",
    "el_state_id": 0,
    "field_path": "count",
    "template": "Clicked %VALUE% Times",
  }
]
}
</script>
```

Importantly notice what is *missing* from the output: No js, no wasm, just some json directives to be fed to a (theoretically) battle-hardened client library, resulting in a kind of local-first HTMX.

[Automerge](https://automerge.org/) is an excellent sync-engine (with a very stylish new website), and some [initial prototypes](https://github.com/mrchantey/beet/blob/beet_state/packages/beet_state/src/demos/RenderListDemo.ts) with its `solidjs` layer have shown promise. The next iteration will likely be written in rust/bevy like the rest of beet is, automerge is already a rust wasm binary so we'd just be adding the dom binding layer on top of that. The key difference between this approach and client islands is that this wasm binary is pre-compiled, the user will rarely need to refetch it, even if the site content *or behavior* changes.

Of course there are limitations to this approach. State mutations are constrained to specific verbs like `increment`, `push_form_data`, `set_from_target_value` in a similar way to the HTMX rendering verbs of `innerHTML`, `outerHTML`, etc.
Here we're counting on the HTMX insight: 80% of reactive operations are CRUD-like and do not require custom client code. We can use Astro-style JS sprinkling for special cases and we still have client islands in the back pocket for inherently heavy applications like 3D rendering or robotics dashboards.

There is still a lot of questions around both in performance and developer experience that can only be answered by hacking away at something like this but I think it looks promising. If you'd like to nerd out on this and other metaframework stuff please come and say hi in [our channel in the bevy discord](https://discord.com/channels/691052431525675048/1333204907414523964).


## A Fully ECS Router

Aside from this exploration the stack bevyfication continues. Axum has now been entirely replaced by our own `hyper` layer using `beet_flow` for the router control flow, and inserting the `Request` and `Response` as entities.
The page you're viewing now is compiled, routed and rendered with ecs technology. Still a super experimental space but a fully ECS router allows for some really fun and interesting patterns, for example the outer content for this blog post is inserted by a render-aware middleware layer, an alternative to Astro's collection-template binding pattern.

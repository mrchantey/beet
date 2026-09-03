+++
title = "Teach it new words"
+++

# Teach it new words

In this tutorial we will add behavior to the guestbook that its markup has no vocabulary for, using scripts that live in the same file. By the end you will have minted a component the engine never shipped and written routes that reshape the world through it, without recompiling anything.

## Start from the guestbook

If you are arriving here fresh, put this in a file called `main.bsx` in an empty directory. If you have the guestbook from the previous lesson, you already have it.

```jsx
<CallOnReady {(CliServer, HttpServer{port:8080})}>
<Router>
	<FieldRoute path="sign" field="entries" verb="Push" list/>
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

## A route that is a script

The guestbook greets nobody. Add a route that does, above the page route:

```jsx
	<ScriptRoute path="greet" script="return 'hello ' + (input.params.name ? input.params.name[0] : 'stranger') + ', thanks for visiting'"/>
```

Run it:

```sh
beet greet --name=Ada
```

```text
INFO Get /greet -> 200 OK in 746 µs (#1)
hello Ada, thanks for visiting
```

And without a name:

```sh
beet greet
```

```text
INFO Get /greet -> 200 OK in 764 µs (#1)
hello stranger, thanks for visiting
```

Notice where that behavior lives. It is an attribute, in the same file as the markup, beside the routes it serves. The script runs in QuickJS with nothing of the host to reach for: no filesystem, no network, no environment. Its `input` is the request, a `{ path, params, body }` map, and it answers with `return`: every script is the body of an async function, which is what will make the `await`s below legal.

## Mint a word the engine never shipped

The guestbook should count visits. Nothing in beet has a component called `guestbook.Visits`, so we will make one. Add this as the first child of the router:

```jsx
	<DynamicComponent name="guestbook.Visits" schema="u64"/>
```

That is a component type with no Rust definition behind it, minted when the scene loads. It is a declaration the scene keeps, so a guestbook that mints a word still mints it after being saved and loaded again. The `schema` says what the word means: a visit count is a whole number, and the guestbook will not hold anything else under that name.

## A script that acts

Now a route that uses it. Add this below the greet route:

```jsx
	<ScriptRoute path="visit"
		{ScriptConfig{read:["guestbook.Visits"],write:["guestbook.Visits"]}}
		script="
		const found = await world.entities('guestbook.Visits');
		const counter = found.length ? found[0] : await world.spawn({ 'guestbook.Visits': 0 });
		await world.insert(counter, 'guestbook.Visits', (await world.get(counter, 'guestbook.Visits')) + 1);
		return 'visit ' + (await world.get(counter, 'guestbook.Visits'));
		"/>
```

The count has to outlive a single request, so start the guestbook as a server:

```sh
beet --server=http
```

From a second terminal, visit three times:

```sh
curl localhost:8080/visit
curl localhost:8080/visit
curl localhost:8080/visit
```

```text
visit 1
visit 2
visit 3
```

That script reached into the world and changed it, through a `world` API it was handed: `world.entities` to find, `world.get` to read, `world.spawn` and `world.insert` to write.

Notice that every one of those is awaited, and that each `await` is a real call against the world at that moment. The `world.spawn` on the second line resolves to a real entity, which the third line writes to. The last line reads back the number the line before it wrote. A script sees its own changes as it makes them.

Notice too that the script has no `reply`. What it returns is the answer: a string comes back as plain text, anything else as JSON, and a script that returns nothing answers `null`. It is the same `return` the greet route used, because it is the same kind of script: the only difference between the two is that this one was handed a `world`.

## A script that is told no

`ScriptConfig` names what a script may reach, and the bridge enforces it, not the script. Add a route that is handed a grant it cannot work with:

```jsx
	<ScriptRoute path="tamper"
		{ScriptConfig{read:["guestbook.Visits"],write:["nothing.*"]}}
		script="
		const [counter] = await world.entities('guestbook.Visits');
		try {
			await world.insert(counter, 'guestbook.Visits', 9999);
			return 'tampered';
		} catch (err) { return err.message; }
		"/>
```

Stop the server with Ctrl-C and start it again, then visit twice, tamper, and visit once more:

```sh
curl localhost:8080/visit
curl localhost:8080/visit
curl localhost:8080/tamper
curl localhost:8080/visit
```

```text
visit 1
visit 2
script may not write `guestbook.Visits`: its write filter is include: nothing.*
visit 3
```

The refusal arrived as an error where the script made the call, so the script caught it and answered with the message. The count carried on from two to three, untouched.

Notice that the script could still *read* `guestbook.Visits`; the two halves of the config are separate lists. A script you did not write gets a shorter one. `ScriptConfig` carries the rest of the envelope too: `world:false` withholds the `world` global outright, leaving a script that is provably a pure transform of its input.

Remember that neither script ever held the world. Each one asked, one call at a time, and the host decided every time whether to answer. That is what lets a script be untrusted and useful at the same time.

## A word that means something

We told the guestbook that `guestbook.Visits` is a whole number. Add a route that ignores that, below the tamper route:

```jsx
	<ScriptRoute path="miscount"
		{ScriptConfig{read:["guestbook.Visits"],write:["guestbook.Visits"]}}
		script="
		const [counter] = await world.entities('guestbook.Visits');
		try {
			await world.insert(counter, 'guestbook.Visits', 'lots');
			return 'miscounted';
		} catch (err) { return err.message; }
		"/>
```

Restart the server again, then visit twice, miscount, and visit once more:

```sh
curl localhost:8080/visit
curl localhost:8080/visit
curl localhost:8080/miscount
curl localhost:8080/visit
```

```text
visit 1
visit 2
`guestbook.Visits` does not accept this value: expected u64, got str
visit 3
```

Notice that this route *was* allowed to write `guestbook.Visits`; its config says so. What refused it was the word's own meaning. The refusal arrived exactly as the tamper route's did, where the call was made, and the count carried on untouched.

## What you have built

You have added behavior to a running tool in the tool's own file: a scripted route, a component type the engine has never heard of with a meaning of its own, and routes that reshaped the world through it. The binary did not change once.

You have also met the edge of what a script can do. Those scripts could not open a file, reach the network or read an environment variable, because they hold no authority at all. So the guestbook still forgets everything the moment it stops.

Next, [Extend the engine](/docs/tutorials/guestbook-plugins) gives it a memory, with the one layer that is allowed to have one.

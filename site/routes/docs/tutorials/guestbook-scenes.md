+++
title = "A guestbook from a scene"
+++

# A guestbook from a scene

In this tutorial we will build a guestbook that people can sign, and serve it over the command line, over HTTP and as a terminal app. By the end you will have a running tool described entirely by one file of markup, with no Rust project anywhere.

## Install the engine

Beet ships as a binary that links a library of capabilities and no behavior at all. Install it once:

```sh
cargo install beet-cli
```

Now make a directory for the guestbook and move into it:

```sh
mkdir guestbook
cd guestbook
```

## Write the smallest tool that runs

Create a file called `main.bsx` with this in it:

```jsx
<CallOnReady {CliServer}>
<Router>
	<Route path="/" {FixedPage}>
		<h1>Guestbook</h1>
		<p>Nobody has signed yet.</p>
	</Route>
</Router>
</CallOnReady>
```

Run it:

```sh
beet
```

```text
INFO Get / -> 200 OK in 3 ms

Guestbook

Nobody has signed yet.
```

Notice that we never told `beet` where to look. It walks up from the current directory until it finds a `main.bsx`, builds it, and lets the tree run itself. Notice too that the heading is drawn large and in colour: the page was rendered for a terminal, not converted for one.

## Give the book somewhere to write

A guestbook needs a list of entries, a way to add to it and a way to read it back. Replace the contents of `main.bsx` with this:

```jsx
<CallOnReady {CliServer}>
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

Read the book:

```sh
beet list
```

```text
INFO Get /list -> 200 OK in 192 µs
[]
```

The book is empty, which is correct: nobody has signed. Now sign it:

```sh
beet sign --body='{"name":"Ada","message":"hello"}'
```

```text
INFO Get /sign -> 200 OK in 223 µs
[{"message":"hello","name":"Ada"}]
```

The route answered with the book, so we can see the signature landed. Now read it back:

```sh
beet list
```

```text
INFO Get /list -> 200 OK in 196 µs
[]
```

Empty again. Notice that each `beet` command is a whole process: it starts, answers one request and exits, taking the entries with it. Nothing is wrong, and nothing was lost that was ever meant to be kept. We simply have not yet given the book anywhere to live longer than a command.

## Keep the process alive

Add an HTTP server beside the CLI one:

```jsx
<CallOnReady {(CliServer, HttpServer{port:8080})}>
```

The rest of the file stays exactly as it was. Now start the tool as a server:

```sh
beet --server=http
```

```text
INFO Mini HTTP server listening on http://127.0.0.1:8080
```

The process stays up. Open a second terminal, leave the first one running, and sign the book twice:

```sh
curl -X POST -H 'content-type: application/json' \
  -d '{"name":"Ada","message":"hello"}' localhost:8080/sign
curl -X POST -H 'content-type: application/json' \
  -d '{"name":"Grace","message":"first computer bug"}' localhost:8080/sign
```

```text
[{"message":"hello","name":"Ada"}]
[{"message":"hello","name":"Ada"},{"message":"first computer bug","name":"Grace"}]
```

Each signature comes back with the book it joined. Read it back on its own:

```sh
curl localhost:8080/list
```

```text
[{"message":"hello","name":"Ada"},{"message":"first computer bug","name":"Grace"}]
```

Both signatures are there, and this time they stayed. Now ask for the page instead of the data:

```sh
curl localhost:8080/
```

```text
<h1>Guestbook</h1><ul><li><!--bx-ref="entries.0.name"-->Ada<!--bx-end--> says <!--bx-ref="entries.0.message"-->hello<!--bx-end--></li><li><!--bx-ref="entries.1.name"-->Grace<!--bx-end--> says <!--bx-ref="entries.1.message"-->first computer bug<!--bx-end--></li></ul>…
```

Notice that we did not write a template for that list. `bx:for="entries"` binds the `<ul>` to the same `entries` the two routes address, and the `<li>` inside it is the shape of one row. One list, three views of it: a JSON route, an HTML page, and the terminal render we started with.

Notice the comments around each value too. Those are markers saying which field of the book that piece of text came from, so a browser can update one name in place rather than redrawing the page.

## The same tool as a terminal app

Stop the server with Ctrl-C. Add a terminal server to the same line:

```jsx
<CallOnReady {(CliServer, HttpServer{port:8080}, TuiServer)}>
```

Run it:

```sh
beet --server=tui
```

The guestbook takes over the terminal, heading and entries drawn as a live application. Press Ctrl-C to leave.

Notice which part of the file changed to make that happen: one word, in the list of servers. The routes did not learn that a terminal exists, and the page did not learn to be a terminal app. A server is a way in and out, and the tool behind it is the same tool.

## Reshape it while it runs

This is the part worth slowing down for. Start the server again, this time watching the file:

```sh
beet --server=http --watch
```

Sign it once from your second terminal so there is something on the page:

```sh
curl -X POST -H 'content-type: application/json' \
  -d '{"name":"Ada","message":"hello"}' localhost:8080/sign
curl localhost:8080/
```

```text
<h1>Guestbook</h1><ul><li><!--bx-ref="entries.0.name"-->Ada<!--bx-end--> says <!--bx-ref="entries.0.message"-->hello<!--bx-end--></li></ul>…
```

Now, with the server still running, edit `main.bsx`. Change the heading and add a line under it:

```jsx
	<Route path="/" {FixedPage}>
		<h1>Ada's Guestbook</h1>
		<p>Leave a note.</p>
		<ul bx:for="entries" bx:key="name">
			<li>{@doc:name} says {@doc:message}</li>
		</ul>
	</Route>
```

Save the file and watch the first terminal:

```text
DEBUG repo store changed, reloading: main.bsx
INFO Mini HTTP server listening on http://127.0.0.1:8080
```

Ask for the page again:

```sh
curl localhost:8080/
```

```text
<h1>Ada&apos;s Guestbook</h1><p>Leave a note.</p><ul></ul>…
```

The running tool reshaped itself around the edit. You did not stop it, rebuild it or redeploy it, because there was nothing to rebuild: the tool is the file, and the file changed.

Notice the other thing that happened. The list is empty. Ada's signature was in memory, the scene was rebuilt from its source, and memory went with it.

## What you have built

You have built a guestbook that signs, lists and renders itself, serves the same routes over three interfaces, and rewrites itself while running, out of one file and no code.

You have also found its two edges. The book forgets: entries live in the running process, so they last exactly as long as it does. And the vocabulary ran out somewhere: the guestbook can push, read and render because those are words the engine ships, and it can do nothing the engine has no word for.

Next, [Teach it new words](/docs/tutorials/guestbook-scripts) takes the second edge, adding behavior the scene has no vocabulary for without recompiling anything.

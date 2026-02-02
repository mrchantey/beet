+++
title="The Harvest #8"
created="2026-02-01"
+++

# The Harvest #8 - It's all been done before

<iframe src="https://www.youtube.com/embed/8nokKDoz2_4" title="The Harvest #8 | It's all been done before" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

<br/>
<br/>

My one-year-old has started getting new ideas, wanting to try something he hasn't done before like open a screw-top lid. Naturally things usually dont go as planned the first time which can be frustrating. In these times I find myself reminding him to take it easy by quoting one of my faviorite songs as a ten-year-old.

> *Chill out, what ya yellin' for?*
>
> *Lay back, it's all been done before*
>
> — Avril Lavigne, Complicated

## Input - Process - Output

Advancements in computing interfaces don't repeat but they certainly rhyme, and the more I play with them the less unique they feel.

If I were old enough to start software development in the 70s my first program might have been a UNIX command-line application:

```c
int main(int argc, char *argv[]) {
	printf("Hello %s", argv[1]);
	return 0;
}
```

If instead I began in the 80s it may have been some kind of REPL-style BASIC program:

```sh
10 INPUT "ENTER YOUR NAME"; N$
20 PRINT "Hello "; N$
30 GOTO 10
```

Maybe in the 90s I would have created a simple CGI web server:

```sh
#!/usr/bin/perl
use CGI;

my $q = CGI->new;
my $name = $q->param('name');

print $q->header;
print "<html><body><h1>Hello $name</h1></body></html>";
```

I wasn't around for any of that, my first coding steps in the 2000s was a Visual Basic GUI:

```sh
Private Sub btnSayHello_Click()
  lblOutput.Caption = "Hello " & txtName.Text
End Sub
```

As each of these interfaces has been introduced the `input-process-output` paradigm has remained the same, and will continue to do so with newer interfaces like voice assistants, XR and humanoid robots.

So if thats true the question is `what ya yellin' for?`, why do these changes cause so much disruption? To software the difference between a CLI, Server or GUI is simply a matter of I/O, or at least it should be. Instead we usually find ourselves in a tech stack that does not translate well to the new interface: C is too verbose for UI, BASIC is too heavy for browsers, javascript is too slow for XR, etc, so we start again from scratch.

As somebody with a career split across web, games and robotics I'm tired of starting again, learning entirely new ecosystems just because the interface is different. Rust can run anywhere and does so very well. Bevy ECS can represent any application and does so very well. For me its time to stop starting again.

## Request / Response as IO

The biggest change this month has been the generalization of the Request / Response structure, decoupling it from the server so that it also represents a CLI, REPL, Clanker tool, and soon forms as well. No more Axum, Clap, Rig or other interface-specific libraries.

The primitives of an exchange are now as follows, simplified for brevity:

**Parts**
- headers: `HashMap<String, Vec<String>>`
	- ie http headers, tool caching instructions
- body: `Vec<u8>`
	- ie http body, tool call payload

**Request**
- path: `Vec<String>`
	- ie http url, cli positional arguments, tool name
- params: `HashMap<String, Vec<String>>`
	- ie http query params, cli flags
- parts: `Parts`

**Response**
- status_code: `u32`
	- ie http status, cli exit code, tool error
- parts: `Parts`

With this generalization things can get very weird, for example a http sitemap, cli `--help` flag and clanker tool definition list are now the same thing. Visit [`https://beetstack.dev/?help`](https://beetstack.dev/?help) to see it in action!

The change has removed some massive dependencies from `beet` alongside their bespoke architectures. The website, cli and clanker tools now all share the same exchange and routing patterns.

Beet is a project that is getting simpler as its capabilities expand, it takes a lot of iterations to get there but the result is shaping up to be a very pretty piece of software, just like the giants of rust and bevy that it sits on the shoulders of.

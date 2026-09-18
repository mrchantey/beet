+++
title = "Beet"
+++

<div bx:style="display=Flex flex-direction=Vertical align-items=Center text-align=Center row-gap=Rem(1.5)">
	<h1 class="text-wordmark">Beet</h1>
	<p class="text-title-large">
		<b>An Atmospheric OS for homegrown tech</b>
	</p>
	<div bx:style="max-width=Rem(34.0)">
		<div class="card-filled">
			<h3>🚧 Mind your step! 🚧</h3>
			<p>
				Beet is under construction. If this project is of interest please come and say hi in the
				<a href="https://roomy.space/did:plc:ldv7dtcgryzerqtffzmleeqm">Beet Roomy space</a>.
			</p>
			<div bx:style="display=Flex justify-content=Center align-items=Center column-gap=Rem(1.0)">
				<Link href="https://github.com/mrchantey/beet" variant=ButtonVariant::Outlined>GitHub</Link>
				<Link href="/docs" variant=ButtonVariant::Filled>Get Started</Link>
			</div>
		</div>
	</div>
</div>
<br/>


Traditionally an operating system is built around a machine with siloed opinions about identity, data and behavior. Beet is oriented around people, not hardware, and lets you access your data and tools wherever you like, however you like.

## Malleable

Beet software grows with your needs, offering a gentle slope from user to tinkerer to systems engineer through three layers of malleability:

1. **Plugins** extend the engine itself in compiled Rust.
2. **Scripts** define composable behaviors in sandboxed JavaScript with fine-grained capabilities.
3. **Scenes** describe structure and behavior as data, edited via text editor, gui or agent.

## Agnostic

- One repo runs the same in a browser tab, a terminal, over ssh, in a window and on a microcontroller.
- The same capabilities serve web sites, games, robots, infra deploys and agent harnesses.
- Every layer of the stack is swappable, the renderer, the http server, the blob store, the filesystem. Keep what you love and swap out the rest.

## Decentralized

- Your software is records and blobs in a repo that belongs to your account, with your atproto PDS as its home.
- Local-first by design, so the repo works with no network and syncs when one appears.
- The [scene format](/docs/scenes) is built for standardization, so other software can read what you made.
- Use a git repo as the source of truth if that is how you like to work.

Pete's [Local-First Conf '26 talk](https://youtu.be/eRpMQhOR93U) shows a static site, a web app, a terminal ui, a server, a robot, an infra deploy and an agent harness all running on one beet binary, driven entirely by scenes.

+++
title = "Beet"
+++

<div bx:style="display=Flex flex-direction=Vertical align-items=Center text-align=Center row-gap=Rem(1.5)">
	<h1 class="text-display-medium">Beet</h1>
	<p class="text-title-large">
		<b>A creative tool engine</b>
	</p>
	<div bx:style="max-width=Rem(34.0)">
		<div class="card-filled">
			<h3>🚧 Mind your step! 🚧</h3>
			<p>
				Beet is under construction. If this project is of interest please come and say hi in the
				<a href="https://discord.gg/DcURUQCXtx">Beetmash Discord Server</a>.
			</p>
			<div bx:style="display=Flex justify-content=Center align-items=Center column-gap=Rem(1.0)">
				<Link href="https://github.com/mrchantey/beet" variant=ButtonVariant::Outlined>GitHub</Link>
				<Link href="/docs" variant=ButtonVariant::Filled>Get Started</Link>
			</div>
		</div>
	</div>
</div>
<br/>

Beet helps you build the perfect stack for cross-domain projects. Websites, agents, robots, games and infra are all under one roof with great defaults and deep extensibility.

## Example - Embodied Agents

Beet is a natural fit for distributed systems like embodied agents with a perceive-act loop. A server is used for the resources, a smartphone for the head and an ESP32 for the body. Each part runs a beet app, and the scenes below are the entire behavior.

### Server

The agent lives on the server: a socket server whose routes are the robot's capabilities. Each cycle the model perceives through `interpret-photo`, then acts through the tools it was given. Capabilities the server can't provide itself, like taking photos and driving wheels, forward over the socket to whichever client serves them.

```jsx
<Router {(SocketServer, BootOnLoad, CapabilityServer)}>
	<!-- routable by interpret-photo, not offered to the model -->
	<TakePhoto/>
	<div {RepeatWhileFunctionCallOutput} {CreateThread}>
		<div {Thread} {Sequence}>
			<CreateActor name="System" kind="System">
				<CreatePost text='
You are a small, curious and very emotional floor robot exploring a room.
You perceive the world one photo at a time and act on what you see.
'/>
			</CreateActor>
			<CreateActor name="Robot" kind="Agent" {ModelStreamer{provider:OpenAi}}>
				<InterpretPhoto/>
				<SpeakText/>
				<SetEmotion/>
				<ApplyHeading/>
			</CreateActor>
		</div>
	</div>
</Router>
```

### Head

The head is a web page opened on the phone. The tab connects to the agent and serves the head capabilities straight from the browser: the webcam serves `take-photo`, speech synthesis serves `speak-text` and the face on screen serves `set-emotion`. This is the instrumented debug view.

```jsx
<HtmlDocument>
	<Fragment slot="head">
		<Stylesheet/>
		<ColorSchemeScript/>
		<LiveReloadScript/>
	</Fragment>
	<h1>Perceive-act web head (debug)</h1>
	<p>
		This tab is the robot's head, instrumented: the webcam the agent sees, its spoken
		lines in the log, and the face it shows. The <code>/</code> route is the same head
		with just the face, fullscreen, for a display.
	</p>
	<h2>Face</h2>
	<img id="face" src="/assets/extra/robot-eyes/calm.png" alt="the robot's face" width="240"/>
	<h2>Webcam (what the agent sees)</h2>
	<video id="webcam" autoplay muted playsinline width="320"/>
	<h2>Head log</h2>
	<RenderConsole/>
	<MainBsx src="/examples/perceive_act/head/head.bsx"/>
	<Wasm src="/assets/wasm/beet.wasm"/>
</HtmlDocument>
```

### Body

The body is a small robot chassis with an ESP32 on board. It connects to the agent over the local network and serves `apply-heading`, turning the model's chosen heading into wheel commands.

```jsx
<AgentSocket url="ws://192.168.86.220:8338">
	<Route path="whoami" {WhoAmi}/>
	<Route path="apply-heading" {ApplyHeading}/>
</AgentSocket>
```

The full example, including mocked and 3d-rendered stages for running without hardware, can be found at [examples/perceive_act](https://github.com/mrchantey/beet/tree/main/examples/perceive_act). See the [docs](/docs) for how the pieces fit together.

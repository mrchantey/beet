import { Repo } from "@automerge/automerge-repo";
import { BroadcastChannelNetworkAdapter } from "@automerge/automerge-repo-network-broadcastchannel";
import { IndexedDBStorageAdapter } from "@automerge/automerge-repo-storage-indexeddb";
import { type StateManifest } from "../state_binding";
import { BindContext } from "../state_binding";
import "./style.css";

// Create the Automerge repo with storage and network adapters
const repo = new Repo({
	network: [new BroadcastChannelNetworkAdapter()],
	storage: new IndexedDBStorageAdapter(),
});

// Define the state manifest with all directives
const stateManifest: StateManifest = {
	state_directives: [
		BindContext.handleEvent({
			el_state_id: 0,
			field_path: "count",
			event: "click",
			action: "increment",
		}),
		BindContext.renderText({
			el_state_id: 1,
			field_path: "count",
			template: "The value is %VALUE%",
		}),
	],
};

// Set up the HTML
document.querySelector<HTMLDivElement>("#app")!.innerHTML = `
	<div>
		<h1>Automerge Counter (Declarative)</h1>
		<div class="card">
			<button id="counter" type="button" data-state-id="0">Increment</button>
			<p data-state-id="1">The value is 0</p>
			<script data-state-manifest type="application/json">
			${JSON.stringify(stateManifest)}
			</script>
		</div>
		<p class="read-the-docs">
			Open this page in multiple tabs to see the counter sync in real-time
		</p>
	</div>
`;

BindContext.init(repo).then((bindContext) => bindContext._unsafeUnwrap());

import { Repo } from "@automerge/automerge-repo";
import { BroadcastChannelNetworkAdapter } from "@automerge/automerge-repo-network-broadcastchannel";
import { IndexedDBStorageAdapter } from "@automerge/automerge-repo-storage-indexeddb";
import { type StateManifest } from "../state_binding";
import "./style.css";
import { BindContext } from "../state_binding";

// Create the Automerge repo with storage and network adapters
const repo = new Repo({
	network: [new BroadcastChannelNetworkAdapter()],
	storage: new IndexedDBStorageAdapter(),
});

// Define the main manifest with RenderList directive
const mainManifest: StateManifest = {
	state_directives: [
		BindContext.renderList({
			el_state_id: 0,
			field_path: "todos",
			template_id: 1,
			item_key_path: "id",
		}),
		BindContext.handleEvent({
			el_state_id: 2,
			event: "click",
			action: "increment",
			field_path: "nextId",
		}),
	],
};

// Template manifest (lives inside the template)
const templateManifest: StateManifest = {
	state_directives: [
		BindContext.renderText({
			el_state_id: 10,
			field_path: "text",
			template: "%VALUE%",
		}),
		BindContext.renderText({
			el_state_id: 11,
			field_path: "id",
			template: "ID: %VALUE%",
		}),
		BindContext.handleEvent({
			el_state_id: 12,
			event: "click",
			action: "increment",
			field_path: "clicks",
		}),
		BindContext.renderText({
			el_state_id: 13,
			field_path: "clicks",
			template: "Clicks: %VALUE%",
		}),
	],
};

// Set up the HTML
document.querySelector<HTMLDivElement>("#app")!.innerHTML = `
	<div>
		<h1>RenderList Demo - Todo Items</h1>

		<div class="card">
			<button id="add-todo" type="button" data-state-id="2">Add Todo</button>

			<ul data-state-id="0" style="list-style: none; padding: 0;">
				<!-- Template for each todo item -->
				<template data-state-id="1">
					<li style="border: 1px solid #ccc; margin: 10px 0; padding: 10px; border-radius: 4px;">
						<div>
							<strong data-state-id="10" style="font-size: 1.2em;">Todo Text</strong>
							<small data-state-id="11" style="color: #666; margin-left: 10px;">ID: 0</small>
						</div>
						<button data-state-id="12" type="button" style="margin-top: 5px;">Click Me</button>
						<span data-state-id="13" style="margin-left: 10px;">Clicks: 0</span>
					</li>

					<!-- Template's own manifest -->
					<script data-state-manifest type="application/json">
					${JSON.stringify(templateManifest)}
					</script>
				</template>
			</ul>

			<!-- Main manifest -->
			<script data-state-manifest type="application/json">
			${JSON.stringify(mainManifest)}
			</script>
		</div>

		<p class="read-the-docs">
			Click "Add Todo" to create new items. Each item has its own state!
		</p>
	</div>
`;

const bindContext = new BindContext(repo);

// Initialize with some sample data
bindContext.init().then((result) => {
	if (result.isOk()) {
		// Add initial todos if the document is empty
		bindContext.docHandle?.change((doc: any) => {
			if (!doc.todos) {
				doc.todos = [
					{ id: "1", text: "Learn Automerge", clicks: 0 },
					{ id: "2", text: "Build amazing apps", clicks: 0 },
					{ id: "3", text: "Share with the world", clicks: 0 },
				];
				doc.nextId = 4;
			}
		});
	}
});

// Add a custom handler to actually add todos when button is clicked
// (This is a workaround since we don't have a "push_item" action yet)
const addButton = document.getElementById("add-todo");
if (addButton) {
	addButton.addEventListener("click", () => {
		bindContext.docHandle?.change((doc: any) => {
			if (!doc.nextId) doc.nextId = 1;
			if (!doc.todos) doc.todos = [];

			const newId = String(doc.nextId);
			doc.todos.push({
				id: newId,
				text: `Todo #${newId}`,
				clicks: 0,
			});
			doc.nextId++;
		});
	});
}

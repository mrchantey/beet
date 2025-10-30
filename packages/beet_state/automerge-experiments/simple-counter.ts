import type { DocHandle, DocumentId } from "@automerge/automerge-repo";
import { Repo } from "@automerge/automerge-repo";
import { BroadcastChannelNetworkAdapter } from "@automerge/automerge-repo-network-broadcastchannel";
import { makeDocumentProjection } from "@automerge/automerge-repo-solid-primitives";
import { IndexedDBStorageAdapter } from "@automerge/automerge-repo-storage-indexeddb";
import { createEffect } from "solid-js";
import "./style.css";

interface MyDoc {
  count: number;
}

// Create the Automerge repo with storage and network adapters
const repo = new Repo({
  network: [new BroadcastChannelNetworkAdapter()],
  storage: new IndexedDBStorageAdapter(),
});

// Set up the HTML
document.querySelector<HTMLDivElement>("#app")!.innerHTML = `
  <div>
    <h1>Automerge Counter</h1>
    <div class="card">
      <button id="counter" type="button"></button>
    </div>
    <p class="read-the-docs">
      Open this page in multiple tabs to see the counter sync in real-time
    </p>
  </div>
`;

async function init() {
  // Get the button element
  const button = document.querySelector<HTMLButtonElement>("#counter")!;
  // Get or create the root document
  let rootDocId = localStorage.getItem("rootDocId") as DocumentId | null;
  let docHandle: DocHandle<MyDoc>;

  if (!rootDocId) {
    docHandle = repo.create<MyDoc>();
    localStorage.setItem("rootDocId", docHandle.documentId);
  } else {
    docHandle = await repo.find<MyDoc>(rootDocId);
  }

  // Use Solid primitives for reactivity - makeDocumentProjection works with a handle directly
  const doc = makeDocumentProjection<MyDoc>(docHandle);

  // Use createEffect to update the DOM when the document changes
  createEffect(() => {
    const count = doc.count ?? 0;
    button.innerHTML = `count is: ${count}`;
  });

  // Add click handler to increment the counter
  button.addEventListener("click", () => {
    docHandle.change((d: any) => {
      d.count = (d.count || 0) + 1;
    });
  });
}

init();

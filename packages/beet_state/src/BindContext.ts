import type { DocHandle, DocumentId } from "@automerge/automerge-repo";
import { Repo } from "@automerge/automerge-repo";
import { BroadcastChannelNetworkAdapter } from "@automerge/automerge-repo-network-broadcastchannel";
import { IndexedDBStorageAdapter } from "@automerge/automerge-repo-storage-indexeddb";
import { err, ok, Result } from "neverthrow";
import type {
	HandleEvent,
	RenderList,
	RenderText,
	StateDirective,
	StateManifest,
} from "./directives";
import { bindHandleEvent } from "./directives/HandleEvent";
import { bindRenderList } from "./directives/RenderList";
import { bindRenderText } from "./directives/RenderText";

/**
 * BindContext provides declarative bindings between DOM elements and Automerge documents.
 *
 * Elements can be configured to:
 * - Trigger document updates in response to events (e.g., increment on click)
 * - Automatically update their content when document fields change
 * - Render lists from arrays with template-based item rendering
 *
 * Configuration is done via a `data-state-manifest` script element containing a StateManifest.
 */
export class BindContext {
	public repo: Repo;
	public docHandle: DocHandle<any> | null = null;
	private mutationObserver: MutationObserver | null = null;
	private boundElements = new WeakSet<Element>();
	private disposers: Array<() => void> = [];
	private pathPrefix?: string;

	constructor(repo?: Repo, pathPrefix?: string) {
		this.repo =
			repo ||
			new Repo({
				network: [new BroadcastChannelNetworkAdapter()],
				storage: new IndexedDBStorageAdapter(),
			});
		this.pathPrefix = pathPrefix;
	}

	/**
	 * Create a test instance with document and localStorage cleared
	 */
	static newTest(repo?: Repo): BindContext {
		document.body.innerHTML = "";
		localStorage.clear();
		return new BindContext(repo || new Repo());
	}

	/**
	 * Create a HandleEvent directive configuration
	 */
	static handleEvent(config: Omit<HandleEvent, "kind">): HandleEvent {
		return { ...config, kind: "handle_event" };
	}

	/**
	 * Create a RenderText directive configuration
	 */
	static renderText(config: Omit<RenderText, "kind">): RenderText {
		return { ...config, kind: "render_text" };
	}

	/**
	 * Create a RenderList directive configuration
	 */
	static renderList(config: Omit<RenderList, "kind">): RenderList {
		return { ...config, kind: "render_list" };
	}

	/**
	 * Parse a state manifest from a script element
	 */
	static parseManifest(
		script: HTMLScriptElement,
	): Result<StateManifest, string> {
		try {
			const manifestJson = script.textContent || "";
			const manifest = JSON.parse(manifestJson) as StateManifest;

			if (
				!manifest.state_directives ||
				!Array.isArray(manifest.state_directives)
			) {
				return err(
					"Invalid manifest: missing or invalid state_directives array",
				);
			}

			return ok(manifest);
		} catch (error) {
			return err(
				`Failed to parse manifest JSON: ${error instanceof Error ? error.message : String(error)}`,
			);
		}
	}

	/**
	 * Create a scoped version of this context with a path prefix
	 */
	scoped(prefix: string): BindContext {
		const scopedContext = new BindContext(this.repo, prefix);
		scopedContext.docHandle = this.docHandle;
		scopedContext.mutationObserver = this.mutationObserver;
		scopedContext.boundElements = this.boundElements;
		// Note: disposers are NOT shared - each scoped context manages its own
		return scopedContext;
	}

	/**
	 * Get the full path with prefix applied
	 */
	private getFullPath(path?: string): string | undefined {
		if (!path) {
			return this.pathPrefix;
		}
		if (!this.pathPrefix) {
			return path;
		}
		return `${this.pathPrefix}.${path}`;
	}

	/**
	 * Get a value from a document using a JSON path like "foo.bar[0].baz"
	 * If path is undefined, returns the root document
	 */
	getValueByPath(doc: any, path?: string): any {
		const fullPath = this.getFullPath(path);
		if (!fullPath) {
			return doc;
		}
		// Parse the path to handle both dot notation and bracket notation
		const keys = fullPath.match(/[^.[\]]+/g) || [];
		let value = doc;
		for (const key of keys) {
			if (value === undefined || value === null) {
				return undefined;
			}
			value = value[key];
		}
		return value;
	}

	/**
	 * Set a value in a document using a JSON path like "foo.bar[0].baz"
	 * If path is undefined, does nothing (can't replace root)
	 */
	setValueByPath(doc: any, path: string | undefined, value: any): void {
		const fullPath = this.getFullPath(path);
		if (!fullPath) {
			console.warn("Cannot set value at root document");
			return;
		}
		const keys = fullPath.match(/[^.[\]]+/g) || [];
		if (keys.length === 0) return;

		let current = doc;
		for (let i = 0; i < keys.length - 1; i++) {
			const key = keys[i];
			if (current[key] === undefined) {
				// Determine if next key is numeric (array) or not (object)
				const nextKey = keys[i + 1];
				current[key] = /^\d+$/.test(nextKey) ? [] : {};
			}
			current = current[key];
		}
		current[keys[keys.length - 1]] = value;
	}

	/**
	 * Initialize BindContext by scanning existing elements and setting up MutationObserver
	 */
	async init(rootDocId?: DocumentId): Promise<Result<void, string>> {
		// Get or create the document handle
		if (rootDocId) {
			this.docHandle = await this.repo.find(rootDocId);
		} else {
			const storedDocId = localStorage.getItem(
				"rootDocId",
			) as DocumentId | null;
			if (storedDocId) {
				this.docHandle = await this.repo.find(storedDocId);
			} else {
				this.docHandle = this.repo.create();
				localStorage.setItem("rootDocId", this.docHandle.documentId);
			}
		}

		// Scan existing elements
		const scanResult = this.scanElements(document.body);
		if (scanResult.isErr()) {
			return err(`Failed to scan elements: ${scanResult.error}`);
		}

		// Set up MutationObserver for dynamic elements
		this.mutationObserver = new MutationObserver((mutations) => {
			for (const mutation of mutations) {
				if (mutation.type === "childList") {
					mutation.addedNodes.forEach((node) => {
						if (node.nodeType === Node.ELEMENT_NODE) {
							this.scanElements(node as Element);
						}
					});
				}
			}
		});

		this.mutationObserver.observe(document.body, {
			childList: true,
			subtree: true,
		});

		return ok();
	}

	/**
	 * Scan an element and its children for data-state-manifest attributes
	 */
	private scanElements(root: Element): Result<void, string> {
		// Look for manifest script in root or its descendants
		const manifestScript = root.querySelector(
			'script[data-state-manifest][type="application/json"]',
		) as HTMLScriptElement | null;

		if (!manifestScript) {
			// No manifest found, nothing to bind
			return ok();
		}

		const manifestResult = BindContext.parseManifest(manifestScript);
		if (manifestResult.isErr()) {
			return err(`Failed to parse manifest: ${manifestResult.error}`);
		}

		const manifest = manifestResult.value;

		// Bind each directive to its corresponding element
		for (const directive of manifest.state_directives) {
			const elementResult = this.findElementForDirective(root, directive);
			if (elementResult.isErr()) {
				console.warn(elementResult.error);
				continue;
			}

			const element = elementResult.value;
			const bindResult = this.bindElement(element, directive);
			if (bindResult.isErr()) {
				console.warn(`Failed to bind element: ${bindResult.error}`);
			}
		}

		return ok();
	}

	/**
	 * Find the element corresponding to a directive's el_state_id
	 */
	findElementForDirective(
		root: Element,
		directive: StateDirective,
	): Result<Element, string> {
		const selector = `[data-state-id="${directive.el_state_id}"]`;

		// Check if root matches
		if (root.matches(selector)) {
			return ok(root);
		}

		// Search descendants
		const element = root.querySelector(selector);
		if (!element) {
			return err(
				`Element with data-state-id="${directive.el_state_id}" not found`,
			);
		}

		return ok(element);
	}

	/**
	 * Bind a single element to its directive
	 */
	private bindElement(
		element: Element,
		directive: StateDirective,
	): Result<void, string> {
		// Skip if already bound
		if (this.boundElements.has(element)) {
			return ok();
		}
		this.boundElements.add(element);

		return this.bindDirective(element, directive);
	}

	/**
	 * Bind a directive to an element
	 */
	bindDirective(
		element: Element,
		directive: StateDirective,
	): Result<void, string> {
		if (!this.docHandle) {
			return err("No document handle available");
		}

		let result: Result<{ dispose?: () => void }, string>;

		if (directive.kind === "handle_event") {
			result = bindHandleEvent(element, directive, this);
		} else if (directive.kind === "render_text") {
			result = bindRenderText(element, directive, this);
		} else if (directive.kind === "render_list") {
			result = bindRenderList(element, directive, this);
		} else {
			return err(`Unknown directive kind: ${(directive as any).kind}`);
		}

		if (result.isErr()) {
			return err(result.error);
		}

		if (result.value.dispose) {
			this.disposers.push(result.value.dispose);
		}

		return ok();
	}

	/**
	 * Add a disposer to this context's cleanup list
	 */
	addDisposer(dispose: () => void): void {
		this.disposers.push(dispose);
	}

	/**
	 * Cleanup and disconnect observer
	 */
	destroy(): void {
		// Cleanup all disposers
		for (const dispose of this.disposers) {
			dispose();
		}
		this.disposers = [];

		if (this.mutationObserver) {
			this.mutationObserver.disconnect();
			this.mutationObserver = null;
		}
		this.repo.networkSubsystem.adapters.forEach((adapter) => {
			adapter?.disconnect?.();
		});
	}
}

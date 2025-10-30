import type { DocHandle, DocumentId } from "@automerge/automerge-repo";
import { Repo } from "@automerge/automerge-repo";
import { BroadcastChannelNetworkAdapter } from "@automerge/automerge-repo-network-broadcastchannel";
import { makeDocumentProjection } from "@automerge/automerge-repo-solid-primitives";
import { IndexedDBStorageAdapter } from "@automerge/automerge-repo-storage-indexeddb";
import { createEffect } from "solid-js";
import { err, ok, Result } from "neverthrow";

/**
 * Base type for binding elements to state directives.
 */
export type BindElement = {
	/** ID used to bind the element via data-state-id attribute */
	el_state_id: number;
};

/**
 * Base type for field location information.
 * Contains the common fields needed to locate a field in an Automerge document.
 */
export type FieldLocation = {
	/** Repository identifier (currently unused, reserved for multi-repo support) */
	doc_repo?: string;

	/** Document ID to use (undefined means use the root document) */
	doc_id?: string;

	/** JSON path to the field in the document (e.g., "count", "user.name", "items[0].value"). Undefined means use the root of the document. */
	field_path?: string;
};

/**
 * Defines how a DOM event should effect the specified document field.
 */
export type HandleEvent = BindElement &
	FieldLocation & {
		/** Discriminant for union type */
		kind: "handle_event";

		/** DOM event name to listen for (e.g., "click", "input", "change") */
		event: string;

		/** Action to perform when the event fires */
		action: "increment" | "decrement" | "set";
	};

/**
 * Defines how a DOM element should update when a document field changes.
 */
export type UpdateDom = BindElement &
	FieldLocation & {
		/** Discriminant for union type */
		kind: "update_dom";

		/** Configuration for how to update the DOM */
		onchange: {
			/** Type of onchange handler */
			kind: "set_with";
			/** Template string with %VALUE% placeholder for the field value */
			template: string;
		};
	};

/**
 * Union type for all state directive configurations.
 */
export type StateDirective = HandleEvent | UpdateDom;

/**
 * Manifest containing all state directives for a root element.
 */
export type StateManifest = {
	/** Array of state directives to bind */
	state_directives: StateDirective[];
};

/**
 * Helper type to make certain keys optional
 */
type PartialBy<T, K extends keyof T> = Omit<T, K> & Partial<Pick<T, K>>;

/**
 * Helper object for creating HandleEvent configurations
 */
export const HandleEvent = {
	create(directive: PartialBy<HandleEvent, "kind">): HandleEvent {
		directive.kind = "handle_event";
		return directive as HandleEvent;
	},
};

/**
 * Helper object for creating UpdateDom configurations
 */
export const UpdateDom = {
	create(directive: PartialBy<UpdateDom, "kind">): UpdateDom {
		directive.kind = "update_dom";
		return directive as UpdateDom;
	},
};

/**
 * StateBinder provides declarative bindings between DOM elements and Automerge documents.
 *
 * Elements can be configured to:
 * - Trigger document updates in response to events (e.g., increment on click)
 * - Automatically update their content when document fields change
 *
 * Configuration is done via a `data-state-manifest` script element containing a StateManifest.
 */
export class StateBinder {
	public repo: Repo;
	private docHandle: DocHandle<any> | null = null;
	private mutationObserver: MutationObserver | null = null;
	private boundElements = new WeakSet<Element>();

	constructor(repo?: Repo) {
		this.repo =
			repo ||
			new Repo({
				network: [new BroadcastChannelNetworkAdapter()],
				storage: new IndexedDBStorageAdapter(),
			});
	}

	/**
	 * Get a value from a document using a JSON path like "foo.bar[0].baz"
	 * If path is undefined, returns the root document
	 */
	private getValueByPath(doc: any, path?: string): any {
		if (!path) {
			return doc;
		}
		// Parse the path to handle both dot notation and bracket notation
		const keys = path.match(/[^.[\]]+/g) || [];
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
	private setValueByPath(doc: any, path: string | undefined, value: any): void {
		if (!path) {
			console.warn("Cannot set value at root document");
			return;
		}
		const keys = path.match(/[^.[\]]+/g) || [];
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
	 * Initialize StateBinder by scanning existing elements and setting up MutationObserver
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

		return ok(undefined);
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
			return ok(undefined);
		}

		const manifestResult = this.parseManifest(manifestScript);
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
			this.bindElement(element, directive);
		}

		return ok(undefined);
	}

	/**
	 * Parse the state manifest from a script element
	 */
	private parseManifest(
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
	 * Find the element corresponding to a directive's el_state_id
	 */
	private findElementForDirective(
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
	private bindElement(element: Element, directive: StateDirective): void {
		// Skip if already bound
		if (this.boundElements.has(element)) {
			return;
		}
		this.boundElements.add(element);

		this.bindDirective(element, directive);
	}

	private bindDirective(element: Element, directive: StateDirective): void {
		// Set up event handler if it's a HandleEvent directive
		if (directive.kind === "handle_event") {
			this.bindEvent(element, directive);
		} // Set up onchange handler if it's a UpdateDom directive
		else if (directive.kind === "update_dom") {
			this.bindOnChange(element, directive);
		}
	}

	/**
	 * Bind an event listener to an element
	 */
	private bindEvent(element: Element, config: HandleEvent): void {
		if (!this.docHandle) return;

		element.addEventListener(config.event, () => {
			this.handleAction(config);
		});
	}

	/**
	 * Handle an action (increment, decrement, set)
	 */
	private handleAction(config: HandleEvent): void {
		if (!this.docHandle) return;

		this.docHandle.change((doc: any) => {
			const fieldPath = config.field_path;

			switch (config.action) {
				case "increment":
					{
						const currentValue = this.getValueByPath(doc, fieldPath) || 0;
						this.setValueByPath(doc, fieldPath, currentValue + 1);
					}
					break;
				case "decrement":
					{
						const currentValue = this.getValueByPath(doc, fieldPath) || 0;
						this.setValueByPath(doc, fieldPath, currentValue - 1);
					}
					break;
				case "set":
					throw new Error("todo");
					// For set, we'd need a value in the config
					break;
			}
		});
	}

	/**
	 * Bind an onchange handler using Solid effects
	 */
	private bindOnChange(element: Element, config: UpdateDom): void {
		if (!this.docHandle) return;

		// Initialize the field if it doesn't exist
		const fieldPath = config.field_path;
		const currentDoc = this.docHandle.doc();
		if (
			currentDoc &&
			this.getValueByPath(currentDoc, fieldPath) === undefined
		) {
			this.docHandle.change((doc: any) => {
				if (this.getValueByPath(doc, fieldPath) === undefined) {
					this.setValueByPath(doc, fieldPath, 0);
				}
			});
		}

		const docProjection = makeDocumentProjection(this.docHandle);

		createEffect(() => {
			const value = this.getValueByPath(docProjection as any, fieldPath) ?? 0;

			if (config.onchange.kind === "set_with") {
				const template = config.onchange.template;
				const text = template.replace("%VALUE%", String(value));
				element.textContent = text;
			}
		});
	}

	/**
	 * Cleanup and disconnect observer
	 */
	destroy(): void {
		if (this.mutationObserver) {
			this.mutationObserver.disconnect();
			this.mutationObserver = null;
		}
		this.repo.networkSubsystem.adapters.forEach((adapter) => {
			adapter?.disconnect?.();
		});
	}
}

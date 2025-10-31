import { ok, err, type Result } from "neverthrow";
import { BindContext } from "../BindContext";
import type {
	BindElement,
	BindResult,
	FieldLocation,
	StateDirective,
} from "./types";

/**
 * Defines how a template should be instantiated for each item in an array.
 * The template contains its own isolated manifest script.
 */
export type RenderList = BindElement &
	FieldLocation & {
		/** Discriminant for union type */
		kind: "render_list";

		/** Reference to the template element by data-state-id */
		template_id: number;

		/**
		 * Optional key path within each item for stable identity
		 * (e.g., "id" would use item.id as the key)
		 * If omitted, uses array index
		 */
		item_key_path?: string;
	};

/**
 * Represents a single item instance in a rendered list
 */
type ListItemInstance = {
	/** Unique key for this item (from item_key_path or index) */
	key: string;
	/** The DOM elements created from the template for this item */
	elements: Element[];
	/** Index in the array for tracking */
	index: number;
	/** Cleanup function to dispose of reactive effects (optional) */
	dispose?: () => void;
};

/**
 * Tracks all instances for a single RenderList directive
 */
type ListInstance = {
	/** The RenderList directive configuration */
	directive: RenderList;
	/** The template element */
	template: HTMLTemplateElement;
	/** Map of item key to item instance */
	items: Map<string, ListItemInstance>;
	/** Container element */
	container: Element;
	/** Array field path */
	arrayPath: string;
	/** Doc change cleanup */
	disposeChangeListener?: () => void;
};

// Global tracking for list instances
const listInstances = new WeakMap<Element, ListInstance>();

/**
 * Bind a RenderList directive to a container element
 */
export function bindRenderList(
	container: Element,
	directive: RenderList,
	context: BindContext,
): Result<BindResult, string> {
	// Find the template element
	const templateResult = context.findElementForDirective(container, {
		...directive,
		el_state_id: directive.template_id,
	} as any);

	if (templateResult.isErr()) {
		return err(
			`Template with data-state-id="${directive.template_id}" not found`,
		);
	}

	const templateElement = templateResult.value;
	if (!(templateElement instanceof HTMLTemplateElement)) {
		return err(
			`Element with data-state-id="${directive.template_id}" is not a <template>`,
		);
	}

	// Initialize the array field if it doesn't exist
	const fieldPath = directive.field_path;

	let currentDoc = context.docHandle!.doc();
	if (
		currentDoc &&
		context.getValueByPath(currentDoc, fieldPath) === undefined
	) {
		context.docHandle!.change((doc: any) => {
			if (context.getValueByPath(doc, fieldPath) === undefined) {
				context.setValueByPath(doc, fieldPath, []);
			}
		});
	}

	// Initialize list instance tracking
	const listInstance: ListInstance = {
		directive,
		template: templateElement,
		items: new Map(),
		container,
		arrayPath: fieldPath || "",
	};

	listInstances.set(container, listInstance);

	// Do initial reconciliation with current state
	currentDoc = context.docHandle!.doc();
	if (currentDoc) {
		const initialArray = context.getValueByPath(currentDoc, fieldPath) ?? [];
		if (Array.isArray(initialArray)) {
			reconcileList(
				container,
				listInstance,
				initialArray,
				fieldPath || "",
				context,
			);
		}
	}

	// Set up doc change listener to trigger reconciliation
	const changeHandler = ({ doc }: any) => {
		// Reconcile all list instances
		for (const [_containerEl, instance] of Array.from(
			document.querySelectorAll("[data-state-id]"),
		).map((el) => [el, listInstances.get(el)] as const)) {
			if (instance) {
				const array = context.getValueByPath(doc, instance.arrayPath) ?? [];
				if (Array.isArray(array)) {
					reconcileList(
						instance.container,
						instance,
						array,
						instance.arrayPath,
						context,
					);
				}
			}
		}
	};

	context.docHandle!.on("change", changeHandler);
	listInstance.disposeChangeListener = () => {
		context.docHandle!.off("change", changeHandler);
	};

	return ok({
		dispose: () => {
			listInstance.disposeChangeListener?.();
			// Cleanup all items
			for (const [_, item] of listInstance.items) {
				item.dispose?.();
			}
			listInstances.delete(container);
		},
	});
}

/**
 * Reconcile the DOM to match the current array state
 */
function reconcileList(
	container: Element,
	listInstance: ListInstance,
	array: any[],
	arrayPath: string,
	context: BindContext,
): void {
	const { directive, template, items } = listInstance;
	const keyPath = directive.item_key_path;

	// Build map of current keys
	const currentKeys = new Set<string>();
	const newItemsOrder: string[] = [];

	for (let i = 0; i < array.length; i++) {
		const item = array[i];
		const key = keyPath
			? String(context.getValueByPath(item, keyPath) ?? i)
			: String(i);

		currentKeys.add(key);
		newItemsOrder.push(key);

		// Create new item instance if it doesn't exist
		if (!items.has(key)) {
			const itemInstanceResult = createListItem(
				template,
				item,
				i,
				arrayPath,
				key,
				context,
			);

			if (itemInstanceResult.isErr()) {
				console.warn(`Failed to create list item: ${itemInstanceResult.error}`);
				continue;
			}

			const itemInstance = itemInstanceResult.value;
			items.set(key, itemInstance);

			// Insert into DOM
			// Find the correct position - insert before the next item or at end
			let insertBefore: Element | null = null;
			for (let j = i + 1; j < newItemsOrder.length; j++) {
				const nextKey = newItemsOrder[j];
				const nextInstance = items.get(nextKey);
				if (nextInstance && nextInstance.elements.length > 0) {
					insertBefore = nextInstance.elements[0];
					break;
				}
			}

			// Insert all elements from this item
			for (const element of itemInstance.elements) {
				if (insertBefore) {
					container.insertBefore(element, insertBefore);
				} else {
					container.appendChild(element);
				}
			}
		}
	}

	// Remove items that are no longer in the array
	const keysToRemove: string[] = [];
	for (const [key, itemInstance] of items.entries()) {
		if (!currentKeys.has(key)) {
			keysToRemove.push(key);
			// Remove from DOM
			for (const element of itemInstance.elements) {
				element.remove();
			}
			// Cleanup reactive effects
			itemInstance.dispose?.();
		}
	}

	for (const key of keysToRemove) {
		items.delete(key);
	}
}

/**
 * Create a new list item instance by cloning the template
 */
function createListItem(
	template: HTMLTemplateElement,
	_item: any,
	index: number,
	arrayPath: string,
	key: string,
	context: BindContext,
): Result<ListItemInstance, string> {
	// Clone the template content
	const fragment = template.content.cloneNode(true) as DocumentFragment;

	// Find the manifest script within the template
	const manifestScript = template.content.querySelector(
		'script[data-state-manifest][type="application/json"]',
	) as HTMLScriptElement | null;

	const elements: Element[] = [];

	// Collect all top-level elements from the fragment (excluding script tags)
	const fragmentChildren = Array.from(fragment.children).filter(
		(el) => !(el instanceof HTMLScriptElement),
	);
	elements.push(...fragmentChildren);

	// Create a scoped context for this list item
	const itemPath = `${arrayPath}[${index}]`;
	const scopedContext = context.scoped(itemPath);

	// If there's a manifest, bind the cloned elements
	if (manifestScript) {
		const manifestResult = BindContext.parseManifest(manifestScript);

		if (manifestResult.isErr()) {
			return err(`Failed to parse template manifest: ${manifestResult.error}`);
		}

		const manifest = manifestResult.value;

		// Bind each directive using the scoped context
		for (const directive of manifest.state_directives) {
			// Find element within the cloned fragment
			const elementResult = findElementInFragmentForDirective(
				elements,
				directive,
			);

			if (elementResult.isErr()) {
				console.warn(
					`Element with data-state-id="${directive.el_state_id}" not found in template instance`,
				);
				continue;
			}

			const element = elementResult.value;

			// Bind the directive with the scoped context
			const bindResult = scopedContext.bindDirective(element, directive);

			if (bindResult.isErr()) {
				console.warn(`Failed to bind directive: ${bindResult.error}`);
			}
		}
	}

	return ok({
		key,
		elements,
		index,
		dispose:
			scopedContext["disposers"].length > 0
				? () => {
						// Dispose all effects created in the scoped context
						for (const dispose of scopedContext["disposers"]) {
							dispose();
						}
					}
				: undefined,
	});
}

/**
 * Find the element corresponding to a directive's el_state_id within fragment elements
 */
function findElementInFragmentForDirective(
	elements: Element[],
	directive: StateDirective,
): Result<Element, string> {
	const selector = `[data-state-id="${directive.el_state_id}"]`;

	for (const el of elements) {
		if (el.matches(selector)) {
			return ok(el);
		}
		const found = el.querySelector(selector);
		if (found) {
			return ok(found);
		}
	}

	return err(`Element with data-state-id="${directive.el_state_id}" not found`);
}

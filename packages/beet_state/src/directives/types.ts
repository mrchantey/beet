import type { HandleEvent } from "./HandleEvent";
import type { RenderList } from "./RenderList";
import type { RenderText } from "./RenderText";

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
 * Union type for all state directive configurations.
 */
export type StateDirective = HandleEvent | RenderText | RenderList;

/**
 * Manifest containing all state directives for a root element.
 */
export type StateManifest = {
	/** Array of state directives to bind */
	state_directives: StateDirective[];
};

/**
 * Result of binding a directive
 */
export type BindResult = {
	/** Optional cleanup function */
	dispose?: () => void;
};

import { createRoot, createEffect } from "solid-js";
import { makeDocumentProjection } from "@automerge/automerge-repo-solid-primitives";
import { ok, type Result } from "neverthrow";
import type {
	BindElement,
	BindResult,
	DirectiveContext,
	FieldLocation,
	PartialBy,
} from "./types";

/**
 * Defines how a DOM element's text should update when a document field changes.
 */
export type RenderText = BindElement &
	FieldLocation & {
		/** Discriminant for union type */
		kind: "render_text";

		/** Template string with %VALUE% placeholder for the field value */
		template: string;
	};

/**
 * Helper function for creating RenderText configurations
 */
export function createRenderText(
	directive: PartialBy<RenderText, "kind">,
): RenderText {
	directive.kind = "render_text";
	return directive as RenderText;
}

/**
 * Bind a RenderText directive to an element
 */
export function bindRenderText(
	element: Element,
	config: RenderText,
	context: DirectiveContext,
): Result<BindResult, string> {
	const { docHandle, getValueByPath, setValueByPath } = context;

	// Initialize the field if it doesn't exist
	const fieldPath = config.field_path;
	const currentDoc = docHandle.doc();
	if (currentDoc && getValueByPath(currentDoc, fieldPath) === undefined) {
		docHandle.change((doc: any) => {
			if (getValueByPath(doc, fieldPath) === undefined) {
				setValueByPath(doc, fieldPath, 0);
			}
		});
	}

	const docProjection = makeDocumentProjection(docHandle);

	const dispose = createRoot((dispose) => {
		createEffect(() => {
			const value = getValueByPath(docProjection as any, fieldPath) ?? 0;
			const template = config.template;
			const text = template.replace("%VALUE%", String(value));
			element.textContent = text;
		});
		return dispose;
	});

	return ok({ dispose });
}

/**
 * Bind a RenderText directive with disposer tracking (for list items)
 */
export function bindRenderTextScoped(
	element: Element,
	config: RenderText,
	context: DirectiveContext,
	disposers: Array<() => void>,
): Result<BindResult, string> {
	const { docHandle, getValueByPath } = context;

	const fieldPath = config.field_path;
	const docProjection = makeDocumentProjection(docHandle);

	const dispose = createRoot((dispose) => {
		createEffect(() => {
			const value = getValueByPath(docProjection as any, fieldPath) ?? "";
			const template = config.template;
			const text = template.replace("%VALUE%", String(value));
			element.textContent = text;
		});
		return dispose;
	});

	disposers.push(dispose);

	return ok({});
}

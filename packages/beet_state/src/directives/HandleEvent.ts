import { err, ok, type Result } from "neverthrow";
import type {
	BindElement,
	BindResult,
	DirectiveContext,
	FieldLocation,
	PartialBy,
} from "./types";

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
 * Helper function for creating HandleEvent configurations
 */
export function createHandleEvent(
	directive: PartialBy<HandleEvent, "kind">,
): HandleEvent {
	directive.kind = "handle_event";
	return directive as HandleEvent;
}

/**
 * Bind a HandleEvent directive to an element
 */
export function bindHandleEvent(
	element: Element,
	config: HandleEvent,
	context: DirectiveContext,
): Result<BindResult, string> {
	const handler = () => {
		handleAction(config, context);
	};

	element.addEventListener(config.event, handler);

	return ok({
		dispose: () => {
			element.removeEventListener(config.event, handler);
		},
	});
}

/**
 * Handle an action (increment, decrement, set)
 */
function handleAction(config: HandleEvent, context: DirectiveContext): void {
	context.docHandle.change((doc: any) => {
		const fieldPath = config.field_path;

		switch (config.action) {
			case "increment":
				{
					const currentValue = context.getValueByPath(doc, fieldPath) || 0;
					context.setValueByPath(doc, fieldPath, currentValue + 1);
				}
				break;
			case "decrement":
				{
					const currentValue = context.getValueByPath(doc, fieldPath) || 0;
					context.setValueByPath(doc, fieldPath, currentValue - 1);
				}
				break;
			case "set":
				return err("set action not yet implemented");
		}
	});
}

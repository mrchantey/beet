interface Message {
	SendEvent?: {
		reg_id: number
		payload: {
			Json: string
		}
	}
}

const messageLookup = {
	"print_registry::MyComponent": 0,
	"print_registry::MyEvent": 1,
	"print_registry::MyResource": 2
}

export function eventMessage(eventName: keyof typeof messageLookup, payload: any): string {
	const reg_id = messageLookup[eventName]
	let message: Message = {
		SendEvent: {
			reg_id,
			payload: {
				Json: JSON.stringify(payload)
			}
		}
	}
	return JSON.stringify(message)
}

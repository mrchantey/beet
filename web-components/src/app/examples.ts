import { AppContext } from "./AppContext"

function titleCase(str: string) {
	str = str.replace(/[-_]/g, ' ')
	return str.replace(/\w\S*/g, val =>
		val.charAt(0).toUpperCase() + val.slice(1).toLowerCase()
	)
}
const src = (name: string): Partial<AppContext> => {
	return {
		appName: titleCase(name),
		src: `https://storage.googleapis.com/beet-examples/${name}/main.js`
	}
}


export const beetExamples: Record<string, Partial<AppContext>> = {
	hello_world: {
		...src('hello_world'),
	},
	seek: {
		...src('seek'),
	},
	seek_3d: {
		...src('seek_3d'),
	},
	flock: {
		...src('flock'),
	},
	hello_ml: {
		...src('hello_ml'),
	},
	animation: {
		...src('animation'),
	},
	fetch: {
		...src('fetch'),
		sidePanel: true,
		loadEvent: true,
		initialPrompt: "I'm hungry!",
	},
}
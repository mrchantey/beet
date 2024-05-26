

export async function runApp(src: string): Promise<undefined> {
	const base = src.startsWith('http') ? undefined : window.location.href
	const noviteSrc = new URL(src, base)
	const module = await import(/* @vite-ignore */noviteSrc.href)
	await module.default().catch((error: Error) => {
		if (error.message.startsWith("Using exceptions for control flow,")) {
			return
		} else {
			throw error
		}
	})
}
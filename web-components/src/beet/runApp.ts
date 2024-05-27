

export async function runApp(src: string): Promise<undefined> {
	const base = src.startsWith('http') ? undefined : window.location.href
	// TODO use import.meta.url? https://vitejs.dev/guide/assets#new-url-url-import-meta-url
	const url = new URL(src, import.meta.url)
	// const url = new URL(src, base)

	const module = await import(/* @vite-ignore */url.href)
	await module.default().catch((error: Error) => {
		if (error.message.startsWith("Using exceptions for control flow,")) {
			return
		} else {
			throw error
		}
	})
}
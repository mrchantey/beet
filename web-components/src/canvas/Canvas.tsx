import { Button, LinearProgress, Typography } from '@suid/material'
import { createSignal, onCleanup, onMount, Show, useContext } from 'solid-js'
import { AppContext } from '../app/AppContext'
import { addEventMessageListener, removeMessageListener, sendStartGameMessage } from '../beet/message'
import { runApp } from '../beet/runApp'
import style from './Canvas.module.css'
export const Canvas = () => {

	const ctx = useContext(AppContext)

	const [loadState, setLoadState] = createSignal('loading')
	const [errorMessage, setErrorMessage] = createSignal<string | null>(null)

	const appReady = () => {
		if (loadState() !== 'loading') {
			return
		}
		if (ctx.startButton) {
			setLoadState('loaded')
		} else {
			startRunning()
		}
	}

	if (ctx.loadEvent) {
		addEventMessageListener("AppReady", appReady)

		onCleanup(() => {
			removeMessageListener(appReady)
		})
	}

	onMount(async () => {
		runApp(ctx.src).then(() => {
			if (!ctx.loadEvent) {
				appReady()
			}
		}).catch(e => {
			setErrorMessage(e.message)
			setLoadState('error')
		})
	})

	const startRunning = () => {
		setLoadState('running')
		sendStartGameMessage()
	}

	return (
		<>
			<div class={style.container}>
				<canvas
					class={style.canvas} id={ctx.canvasId}></canvas>
				<Show when={loadState() !== 'running'}>
					<div class={style.overlay}>
						<Show when={loadState() === 'loading'}>
							<LinearProgress class={style.progressOverlay} />
						</Show>
						<Show when={ctx.startButton && loadState() === 'loaded'}>
							<Button onClick={startRunning}>Start</Button>
						</Show>
						<Show when={errorMessage()}>
							<Typography sx={{ color: theme => theme.palette.error.main }}>{errorMessage()}</Typography>
						</Show>
					</div>
				</Show>
			</div>
		</>
	)
}
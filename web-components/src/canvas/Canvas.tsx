import { Button, LinearProgress, Typography } from '@suid/material'
import { createSignal, onMount, Show, useContext } from 'solid-js'
import { AppContext } from '../app/AppContext'
import { sendStartGameMessage } from '../beet/message'
import { runApp } from '../beet/runApp'
import style from './Canvas.module.css'
export const Canvas = () => {

	const appContext = useContext(AppContext)

	const [customLoad, setCustomLoad] = createSignal(false)
	const [requireInteraction, setRequireInteraction] = createSignal(false)
	const [loadState, setLoadState] = createSignal('loading')
	const [errorMessage, setErrorMessage] = createSignal<string | null>(null)

	onMount(async () => {
		runApp(appContext.src).then(() => {
			if (!customLoad()) {
				finishedLoading()
			}
		}).catch(e => {
			setErrorMessage(e.message)
			setLoadState('error')
		})
	})

	const finishedLoading = () => {
		if (loadState() !== 'loading') {
			return
		}
		if (requireInteraction()) {
			setLoadState('loaded')
		} else {
			startRunning()
		}
	}

	const startRunning = () => {
		setLoadState('running')
		sendStartGameMessage()
	}


	return (
		<>
			<div class={style.container}>
				<canvas
					class={style.canvas} id={appContext.canvasId}></canvas>
				<Show when={loadState() !== 'running'}>
					<div class={style.overlay}>
						<Show when={loadState() === 'loading'}>
							<LinearProgress class={style.progressOverlay} />
						</Show>
						<Show when={requireInteraction() && loadState() !== 'loaded'}>
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
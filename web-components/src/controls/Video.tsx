import { FormControlLabel, Stack, Switch } from "@suid/material"
import { createEffect, createSignal, Show } from "solid-js"
import { stringifyResult, useGestureDetector } from '../mediapipe/gestureDetector'
import styles from './Controls.module.css'

export const Video = () => {

	const [video, setVideo] = createSignal<HTMLVideoElement>()
	const [canvas, setCanvas] = createSignal<HTMLCanvasElement>()

	let { detector, result } = useGestureDetector(video, canvas)

	createEffect(() => {
		let det = detector()
		if (!det) return
	})


	let handleChange = (_: any, checked: boolean) => {
		if (checked)
			detector()?.start()
		else
			detector()?.stop()
	}

	let resultStr = () => stringifyResult(result())

	return (
		<Stack>
			<FormControlLabel
				// disabled={!detector.ready()}
				onChange={handleChange}
				class={styles.toggleControl}
				labelPlacement="start"
				control={<Switch />} label="Video" />
			<Show when={resultStr}>
				<div>{resultStr()}</div>
			</Show>
			<div class={styles.videoContainer}>
				<video ref={setVideo} autoplay playsinline></video>
				<canvas ref={setCanvas}></canvas>
				{/* <div></div> */}
			</div>
		</Stack>


	)
}
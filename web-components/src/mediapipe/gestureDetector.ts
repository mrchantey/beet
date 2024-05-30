//derived from https://codepen.io/mediapipe-preview/pen/zYamdVd
import {
	DrawingUtils,
	FilesetResolver,
	GestureRecognizer,
	GestureRecognizerResult
} from "@mediapipe/tasks-vision"
import { Accessor, createEffect, createSignal } from "solid-js"



export const useGestureDetector = (
	video: Accessor<HTMLVideoElement | undefined>,
	canvas: Accessor<HTMLCanvasElement | undefined>
) => {

	const [detector, setDetector] = createSignal<GestureDetector>()
	const [ready, setReady] = createSignal(false)
	const [result, setResult] = createSignal<GestureRecognizerResult | null>(null)

	createEffect(async () => {
		const videoEl = video()
		const canvasEl = canvas()
		const detectorEl = detector()
		if (detectorEl || !videoEl || !canvasEl) {
			return
		}
		console.assert(videoEl !== undefined)
		console.assert(canvasEl !== undefined)

		const vision = await FilesetResolver.forVisionTasks(
			"https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@0.10.3/wasm"
		)
		const recognizer = await GestureRecognizer.createFromOptions(vision, {
			runningMode: "VIDEO",
			baseOptions: {
				modelAssetPath:
					"https://storage.googleapis.com/mediapipe-models/gesture_recognizer/gesture_recognizer/float16/1/gesture_recognizer.task",
				delegate: "GPU"
			},
		})

		let newDetector = new GestureDetector(videoEl, canvasEl, recognizer, setResult)
		setReady(true)
		setDetector(newDetector)
	})

	return { detector, ready, result }
}

export class GestureDetector {
	gestureRecognizer: GestureRecognizer
	running: boolean = false
	video: HTMLVideoElement
	canvas: HTMLCanvasElement
	canvasCtx: CanvasRenderingContext2D
	onResult: (result: GestureRecognizerResult) => void
	lastVideoTime = -1

	constructor(
		video: HTMLVideoElement,
		canvas: HTMLCanvasElement,
		recognizer: GestureRecognizer,
		onResult: (result: GestureRecognizerResult) => void,
	) {
		this.video = video
		let ctx = canvas.getContext("2d")
		if (ctx === null) {
			throw new Error("Canvas context is null")
		}
		this.canvasCtx = ctx
		this.gestureRecognizer = recognizer
		this.canvas = canvas
		this.onResult = onResult
	}

	async start() {
		if (!(navigator.mediaDevices && navigator.mediaDevices.getUserMedia)) {
			throw new Error("getUserMedia is not supported")
		}
		let stream = await navigator.mediaDevices.getUserMedia({
			video: true
		})
		this.video.srcObject = stream
		this.video.addEventListener("loadeddata", () => this.predict())

		this.running = true
	}

	stop() {
		this.running = false
		if (this.video.srcObject) {
			let stream = this.video.srcObject as MediaStream
			let tracks = stream.getTracks()
			tracks.forEach((track) => track.stop())
		}
	}

	predict() {
		if (this.running) {
			window.requestAnimationFrame(() => this.predict())
		}
		if (this.video.currentTime === this.lastVideoTime) {
			return
		}
		this.lastVideoTime = this.video.currentTime

		const result = this.gestureRecognizer.recognizeForVideo(this.video, Date.now())

		this.drawResult(result)
		this.onResult(result)
	}

	drawResult(result: GestureRecognizerResult) {
		this.canvasCtx.save()
		this.canvasCtx.clearRect(0, 0, this.canvas.width, this.canvas.height)
		const drawingUtils = new DrawingUtils(this.canvasCtx)
		// const videoHeight = "360px"
		// const videoWidth = "480px"
		if (result?.landmarks) {
			for (const landmarks of result.landmarks) {
				drawingUtils.drawConnectors(
					landmarks,
					GestureRecognizer.HAND_CONNECTIONS,
					{
						color: "#00FF00",
						lineWidth: 1
					}
				)
				// drawingUtils.drawLandmarks(landmarks, {
				// 	color: "#FF0000",
				// 	lineWidth: 1
				// })
			}
		}
		this.canvasCtx.restore()
	}
}

export const stringifyResult = (result: GestureRecognizerResult | null) => {
	if (result?.gestures?.length && result.gestures.length > 0) {
		const categoryName = result.gestures[0][0].categoryName
		const categoryScore = result.gestures[0][0].score * 100
		const handedness = result.handedness[0][0].displayName
		return `GestureRecognizer: ${categoryName}\n Confidence: ${categoryScore.toFixed(2)} %\n Handedness: ${handedness}`
	} else {
		return null
	}
}

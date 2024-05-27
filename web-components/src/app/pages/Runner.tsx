import { Stack, Theme } from '@suid/material'
import { useContext } from 'solid-js'
import { Canvas } from '../../canvas/Canvas'
import { Controls } from '../../controls/Controls'
import { AppContext } from '../AppContext'
import styles from './Layout.module.css'


export const Runner = () => {
	let ctx = useContext(AppContext)

	return (
		<Stack sx={{
			height: ctx.fullHeight ? '100dvh' : '50dvh',
			backgroundColor: (theme: Theme) => theme.palette.background.default,
		}} direction="row" class={ctx.sidePanel ? styles.layout : styles.canvasLayout}>
			<Canvas />
			{ctx.sidePanel && <Controls />}
		</Stack>
	)
}
export default Runner


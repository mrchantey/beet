import { Stack, Theme } from '@suid/material'
import { Canvas } from '../canvas/Canvas'
import { Controls } from '../controls/Controls'
import styles from './Layout.module.css'


type Props = {
	fullHeight: boolean
}

export const Layout = ({ fullHeight }: Props) => {
	fullHeight ??= true

	return (
		// <Paper elevation={5}>
		<Stack sx={{
			position: 'relative',
			height: fullHeight ? '100vh' : 'auto',
			backgroundColor: (theme: Theme) => theme.palette.background.default,
		}} direction="row" class={styles.layout}>
			<Canvas />
			<Controls />
		</Stack>
		// </Paper >
	)
}
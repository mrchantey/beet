// import '@fontsource/roboto/300.css'
// import '@fontsource/roboto/400.css'
// import '@fontsource/roboto/500.css'
// import '@fontsource/roboto/700.css'
import { CssBaseline, ThemeProvider, useTheme } from '@suid/material'
import type { Component } from 'solid-js'
import { AppContext, defaultAppContext } from './AppContext'
import { Layout } from './Layout'
import { customTheme } from './theme'

const App: Component = (props: Partial<AppContext>) => {

  useTheme(customTheme)

  let context = defaultAppContext(props)

  return (
    <ThemeProvider theme={customTheme}>
      <CssBaseline />
      <AppContext.Provider value={context}>
        <Layout fullHeight={context.fullHeight}></Layout>
      </AppContext.Provider>
    </ThemeProvider>
  )
}

export { App }


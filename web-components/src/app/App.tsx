// import '@fontsource/roboto/300.css'
// import '@fontsource/roboto/400.css'
// import '@fontsource/roboto/500.css'
// import '@fontsource/roboto/700.css'
import { CssBaseline, ThemeProvider } from '@suid/material'
import { lazy, type Component } from 'solid-js'
import { AppContext, defaultAppContext } from './AppContext'
import Runner from './pages/Runner'
import { useBeetTheme } from './theme'

const Home = lazy(() => import("./pages/Home"))
const ExampleRunner = lazy(() => import("./pages/ExampleRunner"))
const ForeignRunner = lazy(() => import("./pages/ForeignRunner"))
const NotFound = lazy(() => import("./pages/NotFound"))

const App: Component = (props: Partial<AppContext>) => {
  let theme = useBeetTheme()
  let ctx = defaultAppContext(props)
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <AppContext.Provider value={ctx}>
        {/* <Video /> */}
        <Runner />
        {/* <Router> */}
        {/* <Route path="/" component={Home} /> */}
        {/* <Route path="/examples" component={Home} /> */}
        {/* <Route path="/examples/*name" component={ExampleRunner} /> */}
        {/* <Route path="/foo/bar" component={() => <div>foobar</div>} />
          <Route path="/run" component={Runner} />
          <Route path="*404" component={NotFound} /> */}
        {/* </Router> */}
      </AppContext.Provider>
    </ThemeProvider>
  )
}

export { App }


import DefaultTheme from 'vitepress/theme'
import type { Theme } from 'vitepress'

import CompilerPipelineDiagram from './components/CompilerPipelineDiagram.vue'
import MermaidDiagram from './components/MermaidDiagram.vue'

const theme: Theme = {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component('MermaidDiagram', MermaidDiagram)
    app.component('CompilerPipelineDiagram', CompilerPipelineDiagram)
  },
}

export default theme

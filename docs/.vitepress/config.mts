import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'RR',
  description: 'An optimizing compiler from RR to R',
  base: '/RR/',
  themeConfig: {
    nav: [
      { text: 'Guide', link: '/getting-started' },
      { text: 'Language', link: '/language' },
      { text: 'Internals', link: '/compiler-pipeline' },
    ],

    sidebar: [
      {
        text: 'Guide',
        items: [
          { text: 'Getting Started', link: '/getting-started' },
          { text: 'CLI Reference', link: '/cli' },
          { text: 'Configuration', link: '/configuration' },
        ],
      },
      {
        text: 'Language',
        items: [
          { text: 'Language Reference', link: '/language' },
          { text: 'Compatibility & Limits', link: '/compatibility' },
        ],
      },
      {
        text: 'Internals',
        items: [
          { text: 'Compiler Pipeline', link: '/compiler-pipeline' },
          { text: 'IR Model (HIR & MIR)', link: '/ir-model' },
          { text: 'Tachyon Optimizer', link: '/optimization' },
          { text: 'Runtime & Errors', link: '/runtime-and-errors' },
        ],
      },
      {
        text: 'Development',
        items: [
          { text: 'Testing & QA', link: '/testing' },
        ],
      },
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/levish0/RR' },
    ],

    search: {
      provider: 'local',
    },

    outline: 'deep',
  },
})

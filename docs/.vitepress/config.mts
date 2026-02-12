import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'RR',
  description: 'An optimizing compiler from RR to R',
  base: '/RR/',
  
  lastUpdated: true,

  themeConfig: {
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Guide', link: '/getting-started' },
      { text: 'Reference', link: '/language' },
      { text: 'Internals', link: '/compiler-pipeline' },
    ],

    sidebar: [
      {
        text: 'Overview',
        items: [
          { text: 'Docs Home', link: '/' },
        ],
      },
      {
        text: 'Guide',
        collapsed: false, 
        items: [
          { text: 'Getting Started', link: '/getting-started' },
          { text: 'CLI Reference', link: '/cli' },
          { text: 'Configuration', link: '/configuration' },
        ],
      },
      {
        text: 'Reference',
        collapsed: false,
        items: [
          { text: 'Language Reference', link: '/language' },
          { text: 'Compatibility & Limits', link: '/compatibility' },
        ],
      },
      {
        text: 'Internals',
        collapsed: true, 
        items: [
          { text: 'Compiler Pipeline', link: '/compiler-pipeline' },
          { text: 'IR Model (HIR & MIR)', link: '/ir-model' },
          { text: 'Tachyon Optimizer', link: '/optimization' },
          { text: 'Runtime & Errors', link: '/runtime-and-errors' },
        ],
      },
      {
        text: 'Development',
        collapsed: true,
        items: [
          { text: 'Testing & QA', link: '/testing' },
        ],
      },
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/Feralthedogg/RR' },
    ],

    search: {
      provider: 'local',
    },

    editLink: {
      pattern: 'https://github.com/Feralthedogg/RR/edit/main/docs/:path',
      text: 'Edit this page on GitHub'
    },

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2026-present Feralthedogg'
    },

    outline: 'deep',
  },
})
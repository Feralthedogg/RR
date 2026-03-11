import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'RR',
  description: 'An optimizing compiler from RR to R',
  base: '/RR/',

  lastUpdated: true,

  markdown: {
    languageAlias: {
      rr: 'r',
    },
    languageLabel: {
      rr: 'RR',
    },
  },

  themeConfig: {
    nav: [
      { text: 'Home', link: '/' },
      {
        text: 'Guide',
        items: [
          { text: 'Getting Started', link: '/getting-started' },
          { text: 'Writing RR', link: '/writing-rr' },
          { text: 'CLI', link: '/cli' },
        ],
      },
      {
        text: 'Reference',
        items: [
          { text: 'Language', link: '/language' },
          { text: 'Configuration', link: '/configuration' },
          { text: 'R Interop', link: '/r-interop' },
          { text: 'Compatibility', link: '/compatibility' },
        ],
      },
      {
        text: 'Internals',
        items: [
          { text: 'Compiler Pipeline', link: '/compiler-pipeline' },
          { text: 'IR Model', link: '/ir-model' },
          { text: 'Tachyon Engine', link: '/optimization' },
          { text: 'Runtime & Errors', link: '/runtime-and-errors' },
        ],
      },
      {
        text: 'Development',
        items: [
          { text: 'Testing', link: '/testing' },
          { text: 'Contributing Audit', link: '/contributing-audit' },
        ],
      },
    ],

    sidebar: [
      {
        text: 'Overview',
        items: [
          { text: 'Docs Home', link: '/' },
          { text: 'Getting Started', link: '/getting-started' },
        ],
      },
      {
        text: 'Guide',
        collapsed: false,
        items: [
          { text: 'Getting Started', link: '/getting-started' },
          { text: 'Writing RR for Performance & Safety', link: '/writing-rr' },
          { text: 'CLI Reference', link: '/cli' },
          { text: 'Configuration', link: '/configuration' },
        ],
      },
      {
        text: 'Reference',
        collapsed: false,
        items: [
          { text: 'Language Reference', link: '/language' },
          { text: 'R Interop', link: '/r-interop' },
          { text: 'Configuration', link: '/configuration' },
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
          { text: 'Contributing Audit', link: '/contributing-audit' },
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
      text: 'Edit this page on GitHub',
    },

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2026-present Feralthedogg',
    },

    outline: 'deep',
  },
})

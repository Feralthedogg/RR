import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'RR',
  description: 'User-first docs for writing RR and compiling it to R',
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
          { text: 'What is New in 2.0', link: '/whats-new-2.0' },
          { text: 'RR for R Users', link: '/r-for-r-users' },
          { text: 'Writing RR', link: '/writing-rr' },
          { text: 'CLI', link: '/cli' },
          { text: 'Configuration', link: '/configuration' },
          { text: 'Package Manager Design', link: '/package-manager-design' },
        ],
      },
      {
        text: 'Reference',
        items: [
          { text: 'Language', link: '/language' },
          { text: 'R Interop', link: '/r-interop' },
          { text: 'Compatibility', link: '/compatibility' },
        ],
      },
      {
        text: 'Compiler',
        items: [
          { text: 'Overview', link: '/compiler/' },
          { text: 'Pipeline', link: '/compiler/pipeline' },
          { text: 'IR Model', link: '/compiler/ir-model' },
          { text: 'Optimization', link: '/compiler/optimization' },
          { text: 'SROA', link: '/compiler/sroa' },
          { text: 'Unsafe Boundaries', link: '/compiler/unsafe-boundaries' },
          { text: 'Testing & QA', link: '/compiler/testing' },
          { text: 'Contributing Audit', link: '/compiler/contributing-audit' },
        ],
      },
    ],

    sidebar: {
      '/compiler/': [
        {
          text: 'Architecture',
          items: [
            { text: 'Overview', link: '/compiler/' },
            { text: 'Compiler Pipeline', link: '/compiler/pipeline' },
            { text: 'Parallel Compilation', link: '/compiler/parallel-compilation' },
            { text: 'IR Model (HIR & MIR)', link: '/compiler/ir-model' },
            { text: 'Tachyon Optimizer', link: '/compiler/optimization' },
            { text: 'Runtime & Errors', link: '/compiler/runtime-and-errors' },
          ],
        },
        {
          text: 'Optimization',
          items: [
            { text: 'Tachyon Optimizer', link: '/compiler/optimization' },
            { text: 'Adaptive Phase Ordering', link: '/compiler/adaptive-phase-ordering' },
            { text: 'Compile-Time Reduction', link: '/compiler/compile-time-reduction' },
            { text: 'MIR SROA Design', link: '/compiler/sroa' },
          ],
        },
        {
          text: 'Development',
          items: [
            { text: 'Unsafe Boundaries', link: '/compiler/unsafe-boundaries' },
            { text: 'Testing & QA', link: '/compiler/testing' },
            { text: 'Contributing Audit', link: '/compiler/contributing-audit' },
          ],
        },
      ],
      '/': [
        {
          text: 'Start Here',
          items: [
            { text: 'Docs Home', link: '/' },
            { text: 'What is New in 2.0', link: '/whats-new-2.0' },
            { text: 'Getting Started', link: '/getting-started' },
            { text: 'RR for R Users', link: '/r-for-r-users' },
          ],
        },
        {
          text: 'Guide',
          collapsed: false,
          items: [
            { text: 'Writing RR for Performance & Safety', link: '/writing-rr' },
            { text: 'CLI Reference', link: '/cli' },
            { text: 'What is New in 2.0', link: '/whats-new-2.0' },
            { text: 'Package Manager Design', link: '/package-manager-design' },
          ],
        },
        {
          text: 'Reference',
          collapsed: false,
          items: [
            { text: 'Language Reference', link: '/language' },
            { text: 'Configuration', link: '/configuration' },
            { text: 'R Interop', link: '/r-interop' },
            { text: 'Compatibility & Limits', link: '/compatibility' },
          ],
        },
        {
          text: 'R Interop Packages',
          collapsed: true,
          items: [
            { text: 'Base / Data', link: '/r-interop/base' },
            { text: 'Stats', link: '/r-interop/stats' },
            { text: 'Stats4', link: '/r-interop/stats4' },
            { text: 'Methods', link: '/r-interop/methods' },
            { text: 'Compiler', link: '/r-interop/compiler' },
            { text: 'Utils', link: '/r-interop/utils' },
            { text: 'Tools', link: '/r-interop/tools' },
            { text: 'Parallel', link: '/r-interop/parallel' },
            { text: 'Splines', link: '/r-interop/splines' },
            { text: 'Tcl/Tk', link: '/r-interop/tcltk' },
            { text: 'Graphics / Visualization', link: '/r-interop/graphics' },
            { text: 'IO / Reshape', link: '/r-interop/io-reshape' },
            { text: 'dplyr / tidyr', link: '/r-interop/dplyr' },
          ],
        },
        {
          text: 'Compiler Docs',
          collapsed: false,
          items: [
            { text: 'Compiler Overview', link: '/compiler/' },
            { text: 'Compiler Pipeline', link: '/compiler/pipeline' },
            { text: 'Parallel Compilation', link: '/compiler/parallel-compilation' },
            { text: 'IR Model (HIR & MIR)', link: '/compiler/ir-model' },
            { text: 'Tachyon Optimizer', link: '/compiler/optimization' },
            { text: 'Adaptive Phase Ordering', link: '/compiler/adaptive-phase-ordering' },
            { text: 'Compile-Time Reduction', link: '/compiler/compile-time-reduction' },
            { text: 'MIR SROA Design', link: '/compiler/sroa' },
            { text: 'Runtime & Errors', link: '/compiler/runtime-and-errors' },
            { text: 'Unsafe Boundaries', link: '/compiler/unsafe-boundaries' },
            { text: 'Testing & QA', link: '/compiler/testing' },
            { text: 'Contributing Audit', link: '/compiler/contributing-audit' },
          ],
        },
      ],
    },

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

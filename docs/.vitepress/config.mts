import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Git Versioner',
  description:
    'Documentation for Git Versioner, a Rust CLI and GitHub Action for calculating semantic versions from Git history.',
  base: '/git-versioner/',
  cleanUrls: true,
  lastUpdated: true,
  ignoreDeadLinks: false,
  head: [['link', { rel: 'icon', href: '/git-versioner/docs/public/logo.png' }]],
  themeConfig: {
    logo: '/docs/public/logo.png',
    siteTitle: 'Git Versioner',
    nav: [
      { text: 'Guide', link: '/' },
      {
        text: 'Workflows',
        items: [
          {
            text: 'Trunk based development',
            link: '/trunk_based_development_without_commit_message_incrementing_test_full_workflow',
          },
          {
            text: 'Feature branches',
            link: '/trunk_based_development_without_commit_message_incrementing_test_full_workflow_with_feature_branches',
          },
        ],
      },
    ],
    sidebar: [
      {
        text: 'Guide',
        items: [{ text: 'Overview', link: '/' }],
      },
      {
        text: 'Workflows',
        items: [
          {
            text: 'Trunk based development',
            link: '/trunk_based_development_without_commit_message_incrementing_test_full_workflow',
          },
          {
            text: 'Feature branches',
            link: '/trunk_based_development_without_commit_message_incrementing_test_full_workflow_with_feature_branches',
          },
        ],
      },
    ],
    search: {
      provider: 'local',
    },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/Crown0815/git-versioner' },
    ],
    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2024-present Felix Kröner',
    },
  },
})

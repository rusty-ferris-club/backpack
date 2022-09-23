// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const lightCodeTheme = require('prism-react-renderer/themes/nightOwlLight')
const darkCodeTheme = require('prism-react-renderer/themes/dracula')
const metaDescription =
  'Create starter projects from Github repos, run actions and automate installs'
const tagline = 'Curate and automate your starter projects'
const absoluteImg = 'https://getbackpack.dev/img/logo@4x.png'

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Backpack',
  tagline,
  url: 'https://getbackpack.dev',
  baseUrl: '/',
  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',
  favicon: 'img/favicon.svg',

  stylesheets: [
    'https://fonts.googleapis.com/css2?family=Jost:ital,wght@0,400;0,500;0,700;1,400;1,500;1,700&display=swap',
  ],

  organizationName: 'rusty-ferris-club',
  projectName: 'backpack',

  // Even if you don't use internalization, you can use this field to set useful
  // metadata like html lang. For example, if your site is Chinese, you may want
  // to replace "en" with "zh-Hans".
  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
        },
        theme: {
          customCss: require.resolve('./src/components/theme/custom.css'),
        },
        gtag: {
          trackingID: 'xxx',
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      metadata: [
        {
          name: 'description',
          content: metaDescription,
        },
        { name: 'og:site_name', content: 'Backpack' },
        { name: 'og:type', content: 'website' },
        {
          name: 'og:description',
          content: tagline,
        },
        {
          name: 'og:image',
          content: absoluteImg,
        },
        {
          name: 'og:image:url',
          content: absoluteImg,
        },
        { name: 'twitter:card', content: 'summary' },
        { name: 'twitter:site', content: '@jondot' },
        {
          name: 'twitter:image',
          content: absoluteImg,
        },
        {
          name: 'twitter:description',
          content: tagline,
        },
      ],
      image: absoluteImg,
      navbar: {
        hideOnScroll: true,
        logo: {
          alt: 'Backpack',
          src: 'img/logo-hz-light.svg',
          srcDark: 'img/logo-hz-dark.svg',
        },
        items: [
          {
            type: 'doc',
            docId: 'getting-started/index',
            position: 'right',
            label: 'Docs',
          },
          {
            href: 'https://github.com/rusty-ferris-club/backpack',
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        logo: {
          alt: 'backpack',
          src: 'img/logo-stacked-light.svg',
          srcDark: 'img/logo-stacked-dark.svg',
        },
        links: [
          {
            title: 'Docs',
            items: [
              {
                label: 'Getting Started',
                to: '/docs/getting-started',
              },
              {
                label: 'FAQ',
                to: '/docs/FAQ',
              },
            ],
          },
          {
            title: 'Community',
            items: [
              {
                label: 'Stack Overflow',
                href: 'https://stackoverflow.com/questions/tagged/backpack',
              },
              {
                label: 'Twitter',
                href: 'https://twitter.com/jondot',
              },
            ],
          },
          {
            title: 'More',
            items: [
              {
                label: 'GitHub',
                href: 'https://github.com/rusty-ferris-club/backpack',
              },
            ],
          },
        ],
      },
      prism: {
        theme: lightCodeTheme,
        darkTheme: darkCodeTheme,
      },
    }),
  themes: [
    [
      '@easyops-cn/docusaurus-search-local',
      {
        hashed: true,
        indexBlog: false,
        language: ['en'],
        highlightSearchTermsOnTargetPage: true,
        explicitSearchResultPath: true,
      },
    ],
  ],
}

module.exports = config

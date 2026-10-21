import { defineConfig } from 'vitepress'
import type { DefaultTheme } from 'vitepress'

const englishSidebar: DefaultTheme.SidebarItem[] = [
  {
    text: 'Guide',
    items: [
      { text: 'Getting started', link: '/guide/' },
      { text: 'Configuration', link: '/guide/configuration' },
    ],
  },
  {
    text: 'Reference',
    items: [
      { text: 'Script parsing', link: '/reference/parsing' },
      { text: 'Expressions', link: '/reference/expressions' },
      { text: 'Script formatting', link: '/reference/formatting' },
      { text: 'Language versions', link: '/reference/language-versions' },
      { text: 'Node.js and npm', link: '/reference/node' },
    ],
  },
  {
    text: 'Development',
    items: [
      { text: 'Architecture', link: '/development/' },
      { text: 'Compatibility', link: '/development/compatibility' },
      { text: 'Engine conformance', link: '/development/engine-conformance' },
      { text: 'Syntax audit', link: '/development/syntax-audit' },
      {
        text: 'Formatter acceptance',
        link: '/development/formatter-acceptance',
      },
      { text: 'Implementation plan', link: '/development/implementation-plan' },
    ],
  },
]

const chineseSidebar: DefaultTheme.SidebarItem[] = [
  {
    text: '指南',
    items: [
      { text: '快速开始', link: '/zh/guide/' },
      { text: '配置', link: '/zh/guide/configuration' },
    ],
  },
  {
    text: '参考',
    items: [
      { text: '脚本解析', link: '/zh/reference/parsing' },
      { text: '表达式解析', link: '/zh/reference/expressions' },
      { text: '脚本格式化', link: '/zh/reference/formatting' },
      { text: '语言版本', link: '/zh/reference/language-versions' },
      { text: 'Node.js 与 npm', link: '/zh/reference/node' },
    ],
  },
  {
    text: '开发',
    items: [
      { text: '架构与契约', link: '/zh/development/' },
      { text: '兼容性', link: '/zh/development/compatibility' },
      { text: '引擎一致性', link: '/zh/development/engine-conformance' },
      { text: '语法审计', link: '/zh/development/syntax-audit' },
      { text: '格式化验收', link: '/zh/development/formatter-acceptance' },
      { text: '实施计划', link: '/zh/development/implementation-plan' },
    ],
  },
]

export default defineConfig({
  title: 'gdfmt',
  description: 'A Rust-based GDScript toolchain built on lossless parsing.',
  cleanUrls: true,
  head: [['meta', { name: 'theme-color', content: '#478cbf' }]],
  transformHead({ siteData }) {
    return [
      [
        'link',
        {
          rel: 'icon',
          type: 'image/svg+xml',
          href: `${siteData.base}logo.svg`,
        },
      ],
    ]
  },
  locales: {
    root: {
      label: 'English',
      lang: 'en',
      themeConfig: {
        nav: [
          { text: 'Guide', link: '/guide/' },
          { text: 'Reference', link: '/reference/parsing' },
          { text: 'Development', link: '/development/' },
        ],
        sidebar: englishSidebar,
        outline: { label: 'On this page', level: [2, 3] },
        footer: {
          message:
            'Code released under the MIT License. <a href="https://github.com/ntnyq/gdfmt/blob/main/docs/public/logo.LICENSE.txt">Logo: CC BY 4.0</a>.',
          copyright: 'An independent GDScript toolchain.',
        },
      },
    },
    zh: {
      label: '简体中文',
      lang: 'zh-CN',
      description: '基于 Rust 与无损解析的 GDScript 工具链。',
      themeConfig: {
        nav: [
          { text: '指南', link: '/zh/guide/' },
          { text: '参考', link: '/zh/reference/parsing' },
          { text: '开发', link: '/zh/development/' },
        ],
        sidebar: chineseSidebar,
        outline: { label: '本页目录', level: [2, 3] },
        docFooter: { prev: '上一页', next: '下一页' },
        editLink: {
          pattern: 'https://github.com/ntnyq/gdfmt/edit/main/docs/:path',
          text: '在 GitHub 上编辑此页',
        },
        langMenuLabel: '切换语言',
        navMenuLabel: '主导航',
        mobileMenuLabel: '打开导航菜单',
        extraMenuLabel: '更多选项',
        darkModeSwitchLabel: '外观',
        lightModeSwitchTitle: '切换到浅色模式',
        darkModeSwitchTitle: '切换到深色模式',
        sidebarMenuLabel: '菜单',
        returnToTopLabel: '回到顶部',
        skipToContentLabel: '跳转到内容',
        notFound: {
          title: '页面未找到',
          quote: '这个页面不存在，请从首页继续浏览。',
          linkLabel: '前往首页',
          linkText: '返回首页',
          link: '/zh/',
        },
        footer: {
          message:
            '代码基于 MIT 许可发布。<a href="https://github.com/ntnyq/gdfmt/blob/main/docs/public/logo.LICENSE.txt">Logo：CC BY 4.0</a>。',
          copyright: '独立的 GDScript 工具链。',
        },
      },
      markdown: {
        container: {
          tipLabel: '提示',
          noteLabel: '说明',
          warningLabel: '注意',
          dangerLabel: '警告',
          detailsLabel: '详情',
        },
        codeCopyButton: {
          tooltipText: '复制代码',
          copiedText: '已复制',
        },
      },
    },
  },
  themeConfig: {
    logo: {
      src: '/logo.svg',
      alt: 'gdfmt',
    },
    socialLinks: [
      {
        icon: 'simple-icons:github',
        link: 'https://github.com/ntnyq/gdfmt',
        ariaLabel: 'GitHub',
      },
    ],
    editLink: {
      pattern: 'https://github.com/ntnyq/gdfmt/edit/main/docs/:path',
      text: 'Edit this page on GitHub',
    },
    search: {
      provider: 'local',
      options: {
        locales: {
          zh: {
            translations: {
              button: {
                buttonText: '搜索文档',
                buttonAriaLabel: '搜索文档',
              },
              modal: {
                displayDetails: '显示详细列表',
                resetButtonTitle: '清除搜索',
                backButtonTitle: '返回',
                noResultsText: '没有找到相关结果',
                footer: {
                  selectText: '选择',
                  navigateText: '切换',
                  closeText: '关闭',
                },
              },
            },
          },
        },
      },
    },
  },
})

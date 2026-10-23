import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import type { Server } from 'node:http'
import { extname, resolve, sep } from 'node:path'
import { after, before, describe, test } from 'node:test'
import { chromium } from 'playwright'
import type { Browser, Page } from 'playwright'
import { byteOffsetToPosition } from '../../docs/.vitepress/theme/components/playground/protocol.ts'

test('diagnostic byte offsets account for Unicode and CRLF', () => {
  const source = '# 中文 😀\r\nvar text="😀"; ?\n'
  const prefix = source.slice(0, source.indexOf('?'))
  assert.deepEqual(
    byteOffsetToPosition(source, new TextEncoder().encode(prefix).length),
    {
      lineNumber: 2,
      column: 'var text="😀"; '.length + 1,
    },
  )
})

async function waitForOutput(page: Page): Promise<void> {
  await page.waitForFunction(() => {
    const copy = document.querySelector<HTMLButtonElement>(
      '[aria-label="Copy output"], [aria-label="复制结果"]',
    )
    return copy && !copy.disabled
  })
}

async function editSource(page: Page, source: string): Promise<void> {
  await page
    .locator('[data-editor="source"] .view-lines')
    .click({ position: { x: 30, y: 12 } })
  await page.keyboard.press('ControlOrMeta+A')
  await page.keyboard.insertText(source)
}

async function selectOption(
  page: Page,
  label: string,
  option: string,
): Promise<void> {
  await page.getByRole('combobox', { name: label, exact: true }).click()
  await page.getByRole('option', { name: option, exact: true }).click()
}

async function downloadText(page: Page, name: string): Promise<string> {
  const pending = page.waitForEvent('download')
  await page.getByRole('button', { name, exact: true }).click()
  const download = await pending
  const path = await download.path()
  assert.ok(path)
  return readFile(path, 'utf8')
}

describe(
  'built playground formats locally, applies options, recovers, and supports both locales',
  { timeout: 120_000, concurrency: false },
  () => {
    let server: Server
    let browser: Browser
    let page: Page
    let origin: string
    const errors: string[] = []

    before(async () => {
      const root = resolve('docs/.vitepress/dist')
      const mime: Record<string, string> = {
        '.html': 'text/html',
        '.js': 'text/javascript',
        '.css': 'text/css',
        '.wasm': 'application/wasm',
        '.svg': 'image/svg+xml',
        '.woff2': 'font/woff2',
      }
      server = createServer(async (request, response) => {
        const pathname = new URL(request.url ?? '/', 'http://localhost')
          .pathname
        const file = resolve(
          root,
          `.${pathname.endsWith('/') ? `${pathname}index.html` : extname(pathname) ? pathname : `${pathname}.html`}`,
        )
        if (!file.startsWith(`${root}${sep}`)) {
          response.writeHead(403).end()
          return
        }
        try {
          const bytes = await readFile(file)
          response.setHeader(
            'Content-Type',
            mime[extname(file)] ?? 'application/octet-stream',
          )
          response.end(bytes)
        } catch {
          response.writeHead(404).end()
        }
      })
      await new Promise<void>(done => server.listen(0, '127.0.0.1', done))
      const address = server.address()
      assert.ok(address && typeof address === 'object')
      browser = await chromium.launch()
      page = await browser.newPage({
        viewport: { width: 1440, height: 1000 },
      })
      page.on('pageerror', error => errors.push(error.message))
      origin = `http://127.0.0.1:${address.port}`
    })

    after(async () => {
      await browser?.close()
      if (server?.listening)
        await new Promise<void>((done, reject) =>
          server.close(error => (error ? reject(error) : done())),
        )
      assert.deepEqual(errors, [])
    })

    test('loads real Monaco and WASM without the documentation sidebar', async () => {
      await page.goto(`${origin}/playground`)
      await waitForOutput(page)
      await page.locator('.monaco-editor').nth(1).waitFor()
      assert.equal(await page.locator('.VPSidebar:visible').count(), 0)
      assert.equal(await page.locator('select:visible').count(), 0)
      assert.match(
        await page.getByTestId('format-duration').innerText(),
        /^\d+\.\d{2} ms$/,
      )
      assert.match(
        await downloadText(page, 'Download output'),
        /var speed: float = 300\.0/,
      )
    })

    test('live edits, manual formatting, diagnostics, and recovery', async () => {
      await editSource(page, 'var value=1\n')
      await waitForOutput(page)
      assert.equal(
        await downloadText(page, 'Download output'),
        'var value = 1\n',
      )
      await page.getByRole('switch', { name: 'Format on change' }).click()
      await editSource(page, 'var text="中文 😀"\n')
      assert.equal(
        await page
          .getByRole('button', { name: 'Download output' })
          .isDisabled(),
        true,
      )
      await page.keyboard.press('ControlOrMeta+Enter')
      await waitForOutput(page)
      assert.equal(
        await downloadText(page, 'Download output'),
        'var text = "中文 😀"\n',
      )
      await page.getByRole('switch', { name: 'Format on change' }).click()
      await editSource(page, 'func broken(\n')
      await page.getByRole('region', { name: 'Diagnostics' }).waitFor()
      assert.equal(
        await page.getByRole('button', { name: 'Copy output' }).isDisabled(),
        true,
      )
      await page.getByRole('button', { name: 'Reset source' }).click()
      await waitForOutput(page)
      assert.equal(
        await page.getByRole('region', { name: 'Diagnostics' }).count(),
        0,
      )
    })

    test('select menus support keyboard selection and dismissal', async () => {
      const version = page.getByRole('combobox', {
        name: 'Godot language version',
        exact: true,
      })
      await version.focus()
      await page.keyboard.press('Enter')
      await page.getByRole('listbox').waitFor()
      await page.waitForFunction(
        () => document.activeElement?.getAttribute('role') === 'option',
      )
      await page.keyboard.press('Home')
      await page.waitForFunction(
        () => document.activeElement?.textContent?.trim() === 'Godot 4.7',
      )
      await page.keyboard.press('ArrowDown')
      await page.waitForFunction(
        () => document.activeElement?.textContent?.trim() === 'Godot 4.6',
      )
      await page.keyboard.press('Enter')
      await waitForOutput(page)
      assert.equal((await version.innerText()).trim(), 'Godot 4.6')
      assert.equal(
        await version.evaluate(node => node === document.activeElement),
        true,
      )
      await version.click()
      await page.getByRole('listbox').waitFor()
      await page.keyboard.press('ArrowDown')
      await page.keyboard.press('Escape')
      assert.equal(await page.getByRole('listbox').count(), 0)
      await page.waitForFunction(
        () =>
          document.querySelector('#pg-version')?.textContent?.trim() ===
          'Godot 4.6',
      )
      assert.equal((await version.innerText()).trim(), 'Godot 4.6')
      const config = JSON.parse(
        await downloadText(page, 'Export configuration'),
      )
      assert.equal(config.language.godotVersion, '4.6')
    })

    test('config changes affect exact output and export valid settings', async () => {
      await editSource(page, 'func run():\n\tvar items=[1,2]\n')
      await selectOption(page, 'Indent style', 'Spaces')
      await selectOption(page, 'Indent width', '2')
      await selectOption(page, 'Arrays', 'Multiline')
      await selectOption(page, 'Definition spacing', '0')
      await selectOption(page, 'Definition spacing', 'Preserve')
      await page.getByRole('switch', { name: 'Final newline' }).click()
      await selectOption(page, 'Line ending', 'CRLF')
      await waitForOutput(page)
      const output = await downloadText(page, 'Download output')
      assert.match(output, /\r\n {2}var items = \[\r\n {4}1,/)
      assert.equal(output.endsWith('\n'), false)
      const config = JSON.parse(
        await downloadText(page, 'Export configuration'),
      )
      assert.equal(config.schemaVersion, 1)
      assert.equal(config.format.indentWidth, 2)
      assert.equal(config.format.finalNewline, false)
      assert.equal(config.format.blankLinesAroundDefinitions, 'preserve')
      await page.getByRole('button', { name: 'Reset options' }).click()
      await waitForOutput(page)
      assert.equal(
        (
          await page
            .getByRole('combobox', { name: 'Indent style', exact: true })
            .innerText()
        ).trim(),
        'Tabs',
      )
    })

    test('every example formats, including protected regions', async () => {
      for (const example of [
        'Collections & lambdas',
        'Comments & protected regions',
        'Player controller',
      ]) {
        await selectOption(page, 'Choose example', example)
        await waitForOutput(page)
        assert.equal(
          await page.getByTestId('format-status').innerText(),
          'Formatted successfully',
        )
        const output = await downloadText(page, 'Download output')
        if (example === 'Comments & protected regions')
          assert.ok(output.includes('\t[1,  2,  3],\n\t[10, 20, 30],'))
      }
      await page.getByRole('button', { name: 'Settings', exact: true }).click()
      assert.equal(
        await page
          .getByRole('complementary', { name: 'Formatter settings' })
          .count(),
        0,
      )
      await page.getByRole('button', { name: 'Settings', exact: true }).click()
    })

    test('site language menu switches playground labels and options', async () => {
      await page.getByRole('button', { name: 'Change language' }).click()
      await page.getByRole('link', { name: '简体中文', exact: true }).click()
      await page.waitForURL(`${origin}/zh/playground`)
      await page.getByRole('main', { name: 'GDScript 演练场' }).waitFor()
      await waitForOutput(page)
      assert.equal(
        await page.getByTestId('format-status').innerText(),
        '格式化完成',
      )
      await page
        .getByRole('textbox', { name: 'GDScript 源代码', exact: true })
        .waitFor({ state: 'attached' })
      await page
        .getByRole('switch', { name: '实时格式化', exact: true })
        .waitFor()
      await selectOption(page, '缩进风格', '空格')
      await selectOption(page, '数组布局', '多行')
      await waitForOutput(page)
      const config = JSON.parse(await downloadText(page, '导出配置'))
      assert.equal(config.format.indentStyle, 'space')
      assert.equal(config.format.arrayLayout, 'multiline')
      await page.getByRole('button', { name: '切换语言' }).click()
      await page.getByRole('link', { name: 'English', exact: true }).click()
      await page.waitForURL(`${origin}/playground`)
      await page.getByRole('main', { name: 'GDScript playground' }).waitFor()
      await waitForOutput(page)
      await page
        .getByRole('button', { name: 'Settings', exact: true })
        .waitFor()
      assert.equal(
        await page.getByTestId('format-status').innerText(),
        'Formatted successfully',
      )
    })

    test('Chinese route and mobile layout retain usable controls', async () => {
      await page.setViewportSize({ width: 390, height: 844 })
      await page.goto(`${origin}/zh/playground`)
      await waitForOutput(page)
      await page.getByRole('main', { name: 'GDScript 演练场' }).waitFor()
      await selectOption(page, '选择示例', '集合与 Lambda')
      await waitForOutput(page)
      await selectOption(page, 'Godot 语言版本', 'Godot 4.5')
      await waitForOutput(page)
      assert.equal(await page.locator('.VPSidebar:visible').count(), 0)
      assert.equal(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
        true,
      )
      const input = await page.locator('[data-editor="source"]').boundingBox()
      const output = await page.locator('[data-editor="output"]').boundingBox()
      assert.ok(input && output && output.y >= input.y + input.height)
      assert.match(await downloadText(page, '下载结果'), /extends Node/)
    })

    test('worker load failure is localized and gives a working retry action', async () => {
      for (const locale of [
        {
          path: '/playground',
          retry: 'Reload formatter',
          copy: 'Copy output',
          error: 'The formatter could not load. Retry to reload it.',
        },
        {
          path: '/zh/playground',
          retry: '重新加载格式化器',
          copy: '复制结果',
          error: '格式化器加载失败，请重新加载。',
        },
      ]) {
        await page.route('**/formatter.worker-*.js', route => route.abort())
        await page.goto(`${origin}${locale.path}`)
        await page.getByRole('button', { name: locale.retry }).waitFor()
        assert.ok(
          (await page.getByRole('alert').innerText()).includes(locale.error),
        )
        assert.equal(
          await page.getByRole('button', { name: locale.copy }).isDisabled(),
          true,
        )
        await page.unroute('**/formatter.worker-*.js')
        await page.getByRole('button', { name: locale.retry }).click()
        await waitForOutput(page)
      }
    })
  },
)

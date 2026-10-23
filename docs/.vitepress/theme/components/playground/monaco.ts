import * as monaco from 'monaco-editor/editor/editor.api.js'
import 'monaco-editor/editor/browser/coreCommands.js'
import 'monaco-editor/editor/contrib/find/browser/findController.js'
import 'monaco-editor/editor/contrib/folding/browser/folding.js'
import 'monaco-editor/editor/contrib/bracketMatching/browser/bracketMatching.js'
import EditorWorker from 'monaco-editor/editor/editor.worker.js?worker'

globalThis.MonacoEnvironment = { getWorker: () => new EditorWorker() }

monaco.languages.register({ id: 'gdscript', extensions: ['.gd'] })
monaco.languages.setMonarchTokensProvider('gdscript', {
  keywords: [
    'and',
    'as',
    'assert',
    'await',
    'break',
    'breakpoint',
    'class',
    'class_name',
    'const',
    'continue',
    'elif',
    'else',
    'enum',
    'extends',
    'for',
    'func',
    'if',
    'in',
    'is',
    'match',
    'not',
    'or',
    'pass',
    'preload',
    'return',
    'self',
    'signal',
    'static',
    'super',
    'var',
    'void',
    'when',
    'while',
    'yield',
  ],
  tokenizer: {
    root: [
      [/#.*$/, 'comment'],
      [/"""/, 'string', '@tripleDouble'],
      [/'''/, 'string', '@tripleSingle'],
      [/"([^"\\]|\\.)*"/, 'string'],
      [/'([^'\\]|\\.)*'/, 'string'],
      [/@[a-zA-Z_]\w*/, 'annotation'],
      [/\b(true|false|null)\b/, 'constant'],
      [
        /\b(int|float|bool|String|StringName|Array|Dictionary|Vector2|Vector3|Node|CharacterBody2D)\b/,
        'type',
      ],
      [
        /[a-zA-Z_]\w*/,
        { cases: { '@keywords': 'keyword', '@default': 'identifier' } },
      ],
      [/0[xX][\da-fA-F_]+|\b\d[\d_]*(\.\d[\d_]*)?([eE][+-]?\d+)?/, 'number'],
      [/[{}()[\]]/, '@brackets'],
      [/[+\-*/%=<>!&|^~]+/, 'operator'],
    ],
    tripleDouble: [
      [/"""/, 'string', '@pop'],
      [/./, 'string'],
    ],
    tripleSingle: [
      [/'''/, 'string', '@pop'],
      [/./, 'string'],
    ],
  },
})
monaco.languages.setLanguageConfiguration('gdscript', {
  comments: { lineComment: '#' },
  brackets: [
    ['{', '}'],
    ['[', ']'],
    ['(', ')'],
  ],
  autoClosingPairs: [
    { open: '{', close: '}' },
    { open: '[', close: ']' },
    { open: '(', close: ')' },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
  ],
  indentationRules: {
    increaseIndentPattern: /:\s*(#.*)?$/,
    decreaseIndentPattern: /^\s*(elif|else)\b.*:/,
  },
})

for (const [name, base, background] of [
  ['gdfmt-light', 'vs', '#ffffff'],
  ['gdfmt-dark', 'vs-dark', '#1b1b1f'],
] as const) {
  monaco.editor.defineTheme(name, {
    base,
    inherit: true,
    rules: [
      { token: 'comment', foreground: base === 'vs' ? '4F733B' : '7BA86A' },
      { token: 'keyword', foreground: base === 'vs' ? '8B4698' : 'C792EA' },
      { token: 'annotation', foreground: base === 'vs' ? '8F6418' : 'E8BE78' },
      { token: 'type', foreground: base === 'vs' ? '286A9D' : '8CC5EE' },
    ],
    colors: {
      'editor.background': background,
      'editorLineNumber.foreground': base === 'vs' ? '#6c7078' : '#9396a2',
      'editor.lineHighlightBackground': base === 'vs' ? '#f5f8fb' : '#22232b',
    },
  })
}

export { monaco }

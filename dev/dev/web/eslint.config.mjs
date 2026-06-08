// eslint.config.mjs
// 2026-06-08 TASK-1.5 引入：ESLint 9 flat config
import js from '@eslint/js'
import tseslint from '@typescript-eslint/eslint-plugin'
import tsparser from '@typescript-eslint/parser'
import vue from 'eslint-plugin-vue'
import vueParser from 'vue-eslint-parser'
import prettier from 'eslint-config-prettier'

// 浏览器/Vite 全局（避免 no-undef 误报）
const browserGlobals = {
  // DOM
  window: 'readonly',
  document: 'readonly',
  navigator: 'readonly',
  location: 'readonly',
  history: 'readonly',
  localStorage: 'readonly',
  sessionStorage: 'readonly',
  HTMLElement: 'readonly',
  HTMLInputElement: 'readonly',
  HTMLDivElement: 'readonly',
  HTMLTextAreaElement: 'readonly',
  Element: 'readonly',
  Event: 'readonly',
  KeyboardEvent: 'readonly',
  MouseEvent: 'readonly',
  CustomEvent: 'readonly',
  File: 'readonly',
  Blob: 'readonly',
  FormData: 'readonly',
  FileReader: 'readonly',
  URL: 'readonly',
  URLSearchParams: 'readonly',
  Image: 'readonly',
  Audio: 'readonly',
  // 浏览器 API
  fetch: 'readonly',
  AbortController: 'readonly',
  AbortSignal: 'readonly',
  crypto: 'readonly',
  TextEncoder: 'readonly',
  TextDecoder: 'readonly',
  requestAnimationFrame: 'readonly',
  cancelAnimationFrame: 'readonly',
  setTimeout: 'readonly',
  clearTimeout: 'readonly',
  setInterval: 'readonly',
  clearInterval: 'readonly',
  queueMicrotask: 'readonly',
  alert: 'readonly',
  confirm: 'readonly',
  prompt: 'readonly',
  // 任意 id（被 Vue/Element Plus 等用）
  // Vite
  import: 'readonly',
  import_meta: 'readonly',
}

export default [
  {
    ignores: [
      'dist/**',
      'node_modules/**',
      'dev/**',
      'coverage/**',
      '*.min.*',
      'package-lock.json',
      '*.config.*',
      '.prettierrc.*',
      'android/**',
      'ios/**',
    ],
  },
  js.configs.recommended,

  // TypeScript 规则
  {
    files: ['**/*.ts', '**/*.tsx'],
    languageOptions: {
      parser: tsparser,
      parserOptions: { ecmaVersion: 'latest', sourceType: 'module' },
      globals: browserGlobals,
    },
    plugins: { '@typescript-eslint': tseslint },
    rules: {
      // 基础
      'no-unused-vars': 'off',
      'no-undef': 'off', // TS 已检查
      'no-empty': ['error', { allowEmptyCatch: true }],
      'no-control-regex': 'off', // 终端 ANSI 序列需要控制字符
      'no-redeclare': 'off', // 插件 export default 名字与 import 同名是正常模式
      // TS
      '@typescript-eslint/no-unused-vars': [
        'warn',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_', caughtErrorsIgnorePattern: '^_' },
      ],
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/no-non-null-assertion': 'off',
      'no-console': ['warn', { allow: ['warn', 'error', 'info'] }],
    },
  },

  // Vue 规则
  {
    files: ['**/*.vue'],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        parser: tsparser,
        ecmaVersion: 'latest',
        sourceType: 'module',
        extraFileExtensions: ['.vue'],
      },
      globals: browserGlobals,
    },
    plugins: { vue, '@typescript-eslint': tseslint },
    rules: {
      ...vue.configs['vue3-recommended'].rules,
      // 命名
      'vue/multi-word-component-names': 'warn',
      'vue/component-name-in-template-casing': [
        'error',
        'PascalCase',
        {
          registeredComponentsOnly: false,
          // Vant 组件是 van- 前缀（kebab-case），PascalCase 规则要忽略
          ignores: [
            '/^van-/',
            'router-link',
            'router-view',
            'transition',
            'transition-group',
            'component',
            'keep-alive',
            'suspense',
            'teleport',
          ],
        },
      ],
      'vue/component-definition-name-casing': ['error', 'PascalCase'],
      // 安全
      'vue/no-v-html': 'warn', // 暂不强制
      // 顺序（不是错，先 warn）
      'vue/attributes-order': 'warn',
      // prop
      'vue/require-default-prop': 'off',
      // 关闭 ESLint 基础误报
      'no-unused-vars': 'off',
      'no-undef': 'off',
      'no-empty': ['error', { allowEmptyCatch: true }],
      'no-control-regex': 'off',
      'no-redeclare': 'off',
      // TS 在 .vue 内的规则
      '@typescript-eslint/no-unused-vars': [
        'warn',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_', caughtErrorsIgnorePattern: '^_' },
      ],
      '@typescript-eslint/no-explicit-any': 'warn',
    },
  },

  // Prettier 最后（覆盖格式类规则）
  prettier,
]

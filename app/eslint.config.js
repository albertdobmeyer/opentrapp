import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import react from 'eslint-plugin-react';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import importPlugin from 'eslint-plugin-import';
import unicorn from 'eslint-plugin-unicorn';
import vitest from '@vitest/eslint-plugin';
import globals from 'globals';

const TYPED_RULES_OFF = Object.fromEntries(
  Object.keys(tseslint.configs.strictTypeChecked.at(-1).rules ?? {})
    .filter((rule) => rule.startsWith('@typescript-eslint/'))
    .map((rule) => [rule, 'off']),
);

export default tseslint.config(
  {
    ignores: [
      'dist/**',
      'node_modules/**',
      'src-tauri/**',
      'playwright-report/**',
      'test-results/**',
      'coverage/**',
      'eslint.config.js',
      'vite.config.ts',
      'vitest.config.ts',
      'playwright.config.ts',
      'postcss.config.js',
      'tailwind.config.js',
    ],
  },

  // Base layer for every linted file
  js.configs.recommended,
  ...tseslint.configs.recommended,

  // Application source: strict-type-checked + complexity gates + plugins
  {
    files: ['src/**/*.{ts,tsx}'],
    extends: [
      ...tseslint.configs.strictTypeChecked,
      ...tseslint.configs.stylisticTypeChecked,
    ],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: 'module',
      parserOptions: {
        project: ['./tsconfig.json'],
        tsconfigRootDir: import.meta.dirname,
        ecmaFeatures: { jsx: true },
      },
      globals: {
        ...globals.browser,
        ...globals.es2022,
      },
    },
    settings: {
      react: { version: '18.3' },
      'import/resolver': {
        typescript: { project: './tsconfig.json' },
        node: true,
      },
    },
    plugins: {
      react,
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
      import: importPlugin,
      unicorn,
    },
    rules: {
      ...react.configs.recommended.rules,
      ...react.configs['jsx-runtime'].rules,

      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'warn',

      'react-refresh/only-export-components': ['warn', { allowConstantExport: true }],

      'import/no-cycle': ['error', { maxDepth: 10 }],
      'import/order': [
        'warn',
        {
          groups: [
            'builtin',
            'external',
            'internal',
            ['parent', 'sibling', 'index'],
            'object',
            'type',
          ],
          'newlines-between': 'always',
          alphabetize: { order: 'asc', caseInsensitive: true },
        },
      ],
      'import/no-duplicates': 'error',
      'import/no-self-import': 'error',
      'import/no-useless-path-segments': 'warn',

      // Unicorn: productive subset (skip stylistic-only and false-positive-prone rules)
      'unicorn/better-regex': 'warn',
      'unicorn/catch-error-name': 'warn',
      'unicorn/consistent-destructuring': 'warn',
      'unicorn/consistent-function-scoping': 'warn',
      'unicorn/error-message': 'error',
      'unicorn/escape-case': 'warn',
      'unicorn/explicit-length-check': 'warn',
      'unicorn/new-for-builtins': 'error',
      'unicorn/no-array-callback-reference': 'warn',
      'unicorn/no-array-for-each': 'warn',
      'unicorn/no-array-push-push': 'warn',
      'unicorn/no-await-expression-member': 'warn',
      'unicorn/no-console-spaces': 'warn',
      'unicorn/no-for-loop': 'warn',
      'unicorn/no-instanceof-array': 'error',
      'unicorn/no-lonely-if': 'warn',
      'unicorn/no-nested-ternary': 'warn',
      'unicorn/no-new-array': 'error',
      'unicorn/no-new-buffer': 'error',
      'unicorn/no-static-only-class': 'warn',
      'unicorn/no-typeof-undefined': 'warn',
      'unicorn/no-unnecessary-await': 'warn',
      'unicorn/no-unreadable-array-destructuring': 'warn',
      'unicorn/no-unreadable-iife': 'warn',
      'unicorn/no-useless-fallback-in-spread': 'warn',
      'unicorn/no-useless-length-check': 'warn',
      'unicorn/no-useless-promise-resolve-reject': 'warn',
      'unicorn/no-useless-spread': 'warn',
      // Disabled: too aggressive when a typed callable's signature requires
      // an argument (e.g. vitest mockResolvedValue<T>()), where the explicit
      // `undefined` carries the type and is not actually useless.
      'unicorn/no-useless-undefined': 'off',
      'unicorn/no-zero-fractions': 'warn',
      'unicorn/prefer-add-event-listener': 'warn',
      'unicorn/prefer-array-find': 'warn',
      'unicorn/prefer-array-flat': 'warn',
      'unicorn/prefer-array-flat-map': 'warn',
      'unicorn/prefer-array-some': 'warn',
      'unicorn/prefer-blob-reading-methods': 'warn',
      'unicorn/prefer-date-now': 'warn',
      'unicorn/prefer-default-parameters': 'warn',
      'unicorn/prefer-dom-node-append': 'warn',
      'unicorn/prefer-dom-node-dataset': 'warn',
      'unicorn/prefer-dom-node-remove': 'warn',
      'unicorn/prefer-dom-node-text-content': 'warn',
      'unicorn/prefer-includes': 'warn',
      'unicorn/prefer-modern-dom-apis': 'warn',
      'unicorn/prefer-modern-math-apis': 'warn',
      'unicorn/prefer-negative-index': 'warn',
      'unicorn/prefer-number-properties': 'warn',
      'unicorn/prefer-optional-catch-binding': 'warn',
      'unicorn/prefer-query-selector': 'warn',
      'unicorn/prefer-regexp-test': 'warn',
      'unicorn/prefer-set-has': 'warn',
      'unicorn/prefer-set-size': 'warn',
      'unicorn/prefer-spread': 'warn',
      // Disabled: would emit String.prototype.replaceAll, an ES2021 feature
      // beyond this project's ES2020 tsconfig target. Re-enable after a
      // separate tsconfig target bump.
      'unicorn/prefer-string-replace-all': 'off',
      'unicorn/prefer-string-slice': 'warn',
      'unicorn/prefer-string-starts-ends-with': 'warn',
      'unicorn/prefer-string-trim-start-end': 'warn',
      'unicorn/prefer-ternary': 'warn',
      'unicorn/prefer-type-error': 'warn',
      'unicorn/require-array-join-separator': 'warn',
      'unicorn/require-number-to-fixed-digits-argument': 'warn',
      'unicorn/throw-new-error': 'error',

      // Complexity gates
      complexity: ['warn', 18],
      'max-lines': ['warn', { max: 500, skipBlankLines: true, skipComments: true }],
      'max-lines-per-function': ['warn', { max: 80, skipBlankLines: true, skipComments: true, IIFEs: true }],
      'max-depth': ['warn', 5],
      'max-nested-callbacks': ['warn', 3],
      'max-params': ['warn', 5],

      // TS: tighten beyond strict-type-checked where the gain is concrete
      '@typescript-eslint/no-unnecessary-condition': 'warn',
      '@typescript-eslint/prefer-readonly': 'warn',
      '@typescript-eslint/switch-exhaustiveness-check': 'error',

      // TS: rules whose strict default fights idiomatic React/event code
      '@typescript-eslint/no-misused-promises': [
        'warn',
        { checksVoidReturn: { attributes: false } },
      ],

      // The rules below are demoted from `error` (strict-type-checked default)
      // to `warn` so the lint gate can ratchet by warning count. Each rule is
      // a candidate to promote back to `error` once its existing violations
      // have been cleaned up in a subsequent PR.
      '@typescript-eslint/no-empty-function': 'warn',
      '@typescript-eslint/no-floating-promises': 'warn',
      '@typescript-eslint/no-unsafe-call': 'warn',
      '@typescript-eslint/no-unsafe-member-access': 'warn',
      '@typescript-eslint/no-unsafe-assignment': 'warn',
      '@typescript-eslint/no-unsafe-return': 'warn',
      '@typescript-eslint/no-unsafe-argument': 'warn',
      '@typescript-eslint/restrict-template-expressions': 'warn',
      '@typescript-eslint/require-await': 'warn',
      '@typescript-eslint/no-non-null-assertion': 'warn',
      '@typescript-eslint/no-invalid-void-type': 'warn',
      '@typescript-eslint/no-deprecated': 'warn',
      '@typescript-eslint/prefer-nullish-coalescing': 'warn',
      '@typescript-eslint/prefer-promise-reject-errors': 'warn',
      '@typescript-eslint/no-confusing-void-expression': 'warn',
      '@typescript-eslint/no-unnecessary-type-parameters': 'warn',
      '@typescript-eslint/no-base-to-string': 'warn',
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/use-unknown-in-catch-callback-variable': 'warn',
      '@typescript-eslint/array-type': 'warn',
      '@typescript-eslint/consistent-indexed-object-style': 'warn',
      '@typescript-eslint/no-inferrable-types': 'warn',
      '@typescript-eslint/prefer-regexp-exec': 'warn',
      'react/no-unescaped-entities': 'warn',
      'no-async-promise-executor': 'warn',
      'no-empty': 'warn',
    },
  },

  // Test files (vitest): looser gates, vitest plugin, allow `any` in mocking
  {
    files: ['src/**/*.{test,spec}.{ts,tsx}', 'src/test/**/*.{ts,tsx}'],
    plugins: { vitest },
    languageOptions: {
      globals: {
        ...globals.browser,
        ...vitest.environments.env.globals,
      },
    },
    rules: {
      ...vitest.configs.recommended.rules,
      'max-lines': ['warn', { max: 600, skipBlankLines: true, skipComments: true }],
      'max-lines-per-function': 'off',
      '@typescript-eslint/no-explicit-any': 'off',
      '@typescript-eslint/no-unsafe-assignment': 'off',
      '@typescript-eslint/no-unsafe-call': 'off',
      '@typescript-eslint/no-unsafe-member-access': 'off',
      '@typescript-eslint/unbound-method': 'off',
    },
  },

  // E2E (Playwright): no type-checked rules, no React, looser everything
  {
    files: ['e2e/**/*.{ts,tsx}'],
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    rules: {
      ...TYPED_RULES_OFF,
      'max-lines': 'off',
      'max-lines-per-function': 'off',
    },
  },
);

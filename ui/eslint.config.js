import eslint from '@eslint/js';
import tseslint from '@typescript-eslint/eslint-plugin';
import tsparser from '@typescript-eslint/parser';
import svelte from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';
import globals from 'globals';

export default [
	eslint.configs.recommended,
	{
		ignores: ['build/', '.svelte-kit/', 'node_modules/', '*.config.js', '*.config.ts']
	},
	{
		files: ['**/*.ts'],
		languageOptions: {
			parser: tsparser,
			parserOptions: {
				ecmaVersion: 2022,
				sourceType: 'module'
			},
			globals: {
				...globals.browser,
				...globals.node,
				// DOM/Fetch API types
				RequestInit: 'readonly'
			}
		},
		plugins: {
			'@typescript-eslint': tseslint
		},
		rules: {
			...tseslint.configs.recommended.rules,
			'@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
			'no-unused-vars': 'off'
		}
	},
	{
		files: ['**/*.svelte'],
		languageOptions: {
			parser: svelteParser,
			parserOptions: {
				parser: tsparser,
				extraFileExtensions: ['.svelte']
			},
			globals: {
				...globals.browser
			}
		},
		plugins: {
			svelte,
			'@typescript-eslint': tseslint
		},
		rules: {
			...svelte.configs.recommended.rules,
			'@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
			'no-unused-vars': 'off',
			'no-undef': 'off' // Svelte 5 runes like $state, $derived are global
		}
	}
];

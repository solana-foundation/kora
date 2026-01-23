import solanaConfig from '@solana/eslint-config-solana';

export default [
    ...solanaConfig,
    {
        ignores: ['dist/**', 'node_modules/**', 'coverage/**', 'docs/**', 'docs-html/**'],
    },
    {
        files: ['test/**/*.ts'],
        rules: {
            '@typescript-eslint/no-explicit-any': 'off',
            '@typescript-eslint/no-unsafe-assignment': 'off',
            '@typescript-eslint/no-unsafe-return': 'off',
            '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
        },
    },
];

import solanaFmt from '@solana-config/oxc/oxfmt';
import { defineConfig } from 'oxfmt';

export default defineConfig({
    ...solanaFmt,
    ignorePatterns: ['dist/**', 'coverage/**'],
});

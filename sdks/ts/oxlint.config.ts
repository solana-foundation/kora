import solanaConfig from '@solana-config/oxc/oxlint';
import { defineConfig } from 'oxlint';

export default defineConfig({
    extends: [solanaConfig],
    ignorePatterns: ['dist/**', 'coverage/**'],
    options: { typeAware: true },
});

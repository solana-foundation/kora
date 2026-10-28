import type { Config } from 'jest';

const config: Config = {
    collectCoverage: false, // Set to true when running with --coverage flag
    collectCoverageFrom: ['src/**/*.ts', '!src/**/*.d.ts', '!src/**/*.test.ts', '!src/**/index.ts'],
    coverageDirectory: 'coverage',
    coverageReporters: ['lcov', 'text', 'html'],
    extensionsToTreatAsEsm: ['.ts'],
    moduleNameMapper: {
        '^(\\.{1,2}/.*)\\.js$': '$1',
    },
    preset: 'ts-jest/presets/default-esm',
    testEnvironment: 'node',
    testMatch: ['**/*.test.ts'],
    testPathIgnorePatterns: ['/node_modules/', '/dist/'],
    transform: {
        '^.+\\.ts$': [
            'ts-jest',
            {
                tsconfig: {
                    module: 'es2022',
                },
                useESM: true,
            },
        ],
    },
};

export default config;

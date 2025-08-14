import { KoraClient } from '../src/index.js';

export function runAuthenticationTests(rpcUrl: string = 'http://localhost:8080/') {
    describe('Authentication', () => {
        it('should fail with incorrect API key', async () => {
            const client = new KoraClient({
                rpcUrl,
                apiKey: 'WRONG-API-KEY',
            });

            // Auth failure should result in an error (empty response body causes JSON parse error)
            await expect(client.getConfig()).rejects.toThrow();
        });

        it('should fail with incorrect HMAC secret', async () => {
            const client = new KoraClient({
                rpcUrl,
                hmacSecret: 'WRONG-HMAC-SECRET',
            });

            // Auth failure should result in an error
            await expect(client.getConfig()).rejects.toThrow();
        });

        it('should fail with both incorrect credentials', async () => {
            const client = new KoraClient({
                rpcUrl,
                apiKey: 'WRONG-API-KEY',
                hmacSecret: 'WRONG-HMAC-SECRET',
            });

            // Auth failure should result in an error
            await expect(client.getConfig()).rejects.toThrow();
        });

        it('should succeed with correct credentials', async () => {
            const client = new KoraClient({
                rpcUrl,
                apiKey: 'test-api-key-123',
                hmacSecret: 'test-hmac-secret-456',
            });

            const config = await client.getConfig();
            expect(config).toBeDefined();
            expect(config.fee_payer).toBeDefined();
        });

        it('should fail when no credentials provided but auth is required', async () => {
            const client = new KoraClient({
                rpcUrl,
            });

            // No credentials should fail when auth is enabled
            await expect(client.getConfig()).rejects.toThrow();
        });
    });
}

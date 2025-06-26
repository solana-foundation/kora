export class KoraClient {
    rpcUrl;
    constructor(rpcUrl) {
        this.rpcUrl = rpcUrl;
    }
    async rpcRequest(method, params) {
        const response = await fetch(this.rpcUrl, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                jsonrpc: '2.0',
                id: 1,
                method,
                params,
            }),
        });
        const json = await response.json();
        if ('error' in json) {
            const error = json.error;
            throw new Error(`RPC Error ${error.code}: ${error.message}`);
        }
        return json.result;
    }
    async getConfig() {
        return this.rpcRequest('getConfig', []);
    }
    async getBlockhash() {
        return this.rpcRequest('getBlockhash', []);
    }
    async getSupportedTokens() {
        return this.rpcRequest('getSupportedTokens', []);
    }
    async estimateTransactionFee(transaction, feeToken) {
        return this.rpcRequest('estimateTransactionFee', [transaction, feeToken]);
    }
    async signTransaction(request) {
        return this.rpcRequest('signTransaction', request);
    }
    async signAndSendTransaction(request) {
        return this.rpcRequest('signAndSendTransaction', request);
    }
    async signTransactionIfPaid(request) {
        return this.rpcRequest('signTransactionIfPaid', request);
    }
    async transferTransaction(request) {
        return this.rpcRequest('transferTransaction', request);
    }
}

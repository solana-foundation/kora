
import { createKoraClient } from "./client";

async function main() {
    const client = createKoraClient('http://localhost:8080/');
    try {
        const config = await client.getConfig().send();
        console.log('Kora Config:', config.result);
        const blockhash = await client.getBlockhash().send();
        console.log('Blockhash: ', blockhash.result.blockhash);
    } catch (error) {
        console.error(error);
    }
}

main().catch(e => console.error('Error:', e));
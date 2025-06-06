const { Connection, Keypair, Transaction } = require('@solana/web3.js');
const axios = require('axios');

async function runTransfer() {
  // 1. Set up connection
  const connection = new Connection('https://api.mainnet-beta.solana.com');
  
  // 2. Generate test accounts (or use predefined)
  const sender = Keypair.generate().publicKey.toString();
  const recipient = Keypair.generate().publicKey.toString();
  
  console.log(`Transferring from ${sender} to ${recipient}`);
  
  // 3. Call Kora relayer
  const response = await axios.post('http://localhost:8080/transferTransaction', {
    amount: 1000, // 0.000001 SOL
    token: 'SOL',
    source: sender,
    destination: recipient
  });

  // 4. Submit transaction
  const tx = Transaction.from(Buffer.from(response.data.transaction, 'base64'));
  const txid = await connection.sendRawTransaction(tx.serialize());
  
  console.log(`Transaction submitted: ${txid}`);
  console.log(`Explorer: https://explorer.solana.com/tx/${txid}`);
}

runTransfer().catch(console.error);
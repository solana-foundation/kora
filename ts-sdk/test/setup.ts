import { config } from 'dotenv';
import path from 'path';

// Load environment variables from .env file
config({ path: path.resolve(__dirname, '../.env') });

// Set default values if not present in .env
process.env.KORA_RPC_URL = process.env.KORA_RPC_URL || 'http://localhost:8080/';
process.env.USDC_MINT = process.env.USDC_MINT || '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU'; 
// Amounts are bigint base units everywhere (lamports, micro-USDC). Never a float.

export const LAMPORTS_PER_SOL = 1_000_000_000n;
export const DAY_MS = 86_400_000;

export const ceilDiv = (a, b) => (a + b - 1n) / b;

export const startOfUtcDay = (ms) => ms - (ms % DAY_MS);

const ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

export function base58Encode(bytes) {
  let value = 0n;
  for (const byte of bytes) value = value * 256n + BigInt(byte);
  let encoded = "";
  for (; value > 0n; value /= 58n) encoded = ALPHABET[Number(value % 58n)] + encoded;
  for (let i = 0; i < bytes.length && bytes[i] === 0; i += 1) encoded = `1${encoded}`;
  return encoded;
}

/** Bytes of a base58 text, or null when it is not base58. */
export function base58Decode(text) {
  if (typeof text !== "string" || text.length === 0) return null;
  let value = 0n;
  for (const char of text) {
    const digit = ALPHABET.indexOf(char);
    if (digit < 0) return null;
    value = value * 58n + BigInt(digit);
  }
  const bytes = [];
  for (; value > 0n; value /= 256n) bytes.unshift(Number(value % 256n));
  for (let i = 0; i < text.length && text[i] === "1"; i += 1) bytes.unshift(0);
  return Uint8Array.from(bytes);
}

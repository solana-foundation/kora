import { createHmac } from "node:crypto";
import { Refusal, UnknownOutcome } from "./errors.mjs";

const SIGN_TIMEOUT_MS = 15_000;

/**
 * Kora's JSON-RPC `signTransaction`, authenticated the way Kora checks it: header `x-api-key`,
 * header `x-timestamp` (unix seconds) and header `x-hmac-signature`, the hex HMAC-SHA256 of
 * the timestamp followed by the exact request body, keyed with the HMAC secret.
 *
 * An answer that carries an error means Kora did not sign. Anything else that is not a clean
 * result (a timeout, a dropped connection, a server error) is an unknown outcome: Kora may
 * have signed, so the caller must not try another fee payer.
 */
export function createKoraClient({ url, apiKey, hmacSecret, fetchImpl = fetch, now = Date.now, timeoutMs = SIGN_TIMEOUT_MS }) {
  return {
    /** The base64 transaction with Kora's fee payer signature added. */
    async signTransaction(transactionBase64) {
      const body = JSON.stringify({ id: 1, jsonrpc: "2.0", method: "signTransaction", params: { transaction: transactionBase64 } });
      const timestamp = Math.floor(now() / 1000).toString();
      const signature = createHmac("sha256", hmacSecret).update(timestamp + body).digest("hex");
      let response;
      let text;
      try {
        response = await fetchImpl(url, {
          method: "POST",
          body,
          headers: { "content-type": "application/json", "x-api-key": apiKey, "x-timestamp": timestamp, "x-hmac-signature": signature },
          signal: AbortSignal.timeout(timeoutMs),
        });
        text = await response.text();
      } catch (error) {
        throw new UnknownOutcome(`the relayer did not answer: ${error.message}`);
      }
      let json = null;
      try {
        json = JSON.parse(text);
      } catch {
        // handled below
      }
      if (json?.error) throw new Refusal("kora_rejected", `the relayer refused to sign: ${String(json.error.message ?? "no reason given")}`);
      if (response.status >= 400 && response.status < 500) throw new Refusal("kora_rejected", `the relayer refused the request (${response.status})`);
      const signed = json?.result?.signed_transaction;
      if (response.ok && typeof signed === "string") return signed;
      throw new UnknownOutcome(`the relayer's answer was unclear (${response.status})`);
    },
  };
}

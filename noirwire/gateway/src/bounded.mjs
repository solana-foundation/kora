// A wall-clock bound on every RPC call. web3.js has none of its own, and one request that
// never answers would otherwise hold the run (and the next scheduled one) forever.

export const RPC_TIMEOUT_MS = 20_000;

export class RpcTimeout extends Error {}

/** Wraps `conn` so that every method returning a promise rejects after `timeoutMs`. */
export function bounded(conn, timeoutMs = RPC_TIMEOUT_MS) {
  return new Proxy(conn, {
    get(target, name) {
      const value = target[name];
      if (typeof value !== "function") return value;
      return (...args) => {
        const result = value.apply(target, args);
        if (!(result instanceof Promise)) return result;
        let timer;
        const timeout = new Promise((_, reject) => {
          timer = setTimeout(() => reject(new RpcTimeout(`${String(name)} did not answer in ${timeoutMs} ms`)), timeoutMs);
        });
        return Promise.race([result, timeout]).finally(() => clearTimeout(timer));
      };
    },
  });
}

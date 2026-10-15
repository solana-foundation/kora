import assert from "node:assert/strict";
import { test } from "node:test";
import { isValidMarkupBps, lamportsToMicroUsdc, splitPayment } from "../src/money.mjs";

test("lamports convert to micro-USDC rounded up", () => {
  assert.equal(lamportsToMicroUsdc(5000n, 150_000_000n), 750n);
  assert.equal(lamportsToMicroUsdc(1n, 150_000_000n), 1n);
  assert.equal(lamportsToMicroUsdc(0n, 150_000_000n), 0n);
  assert.equal(lamportsToMicroUsdc(5001n, 150_000_000n), 751n);
});

test("a missing price never converts", () => {
  assert.throws(() => lamportsToMicroUsdc(5000n, 0n), { code: "no_price" });
});

test("the default markup of 10000 bps gives the app 80 percent, NoirWire rounds up", () => {
  const split = splitPayment({ networkCost: 1000n, markupBps: 10_000 });
  assert.deepEqual(split, { networkCost: 1000n, markup: 1000n, noirwire: 200n, app: 800n, total: 2000n });
});

test("the markup is rounded up and the split always adds back to it", () => {
  for (const networkCost of [1n, 3n, 7n, 751n, 12_345n]) {
    for (const markupBps of [0, 1, 333, 10_000, 30_000]) {
      const s = splitPayment({ networkCost, markupBps });
      assert.equal(s.markup, (networkCost * BigInt(markupBps) + 9999n) / 10_000n);
      assert.equal(s.noirwire + s.app, s.markup);
      assert.equal(s.total, networkCost + s.markup);
      assert.ok(s.noirwire >= s.app / 4n);
    }
  }
});

test("a zero markup leaves both shares at zero", () => {
  assert.deepEqual(splitPayment({ networkCost: 750n, markupBps: 0 }), { networkCost: 750n, markup: 0n, noirwire: 0n, app: 0n, total: 750n });
});

test("a tiny markup gives NoirWire the rounded-up share and the app the rest", () => {
  const split = splitPayment({ networkCost: 1n, markupBps: 1 });
  assert.equal(split.markup, 1n);
  assert.equal(split.noirwire, 1n);
  assert.equal(split.app, 0n);
});

test("300 percent is the ceiling", () => {
  assert.equal(isValidMarkupBps(30_000), true);
  assert.equal(isValidMarkupBps(30_001), false);
  assert.equal(isValidMarkupBps(-1), false);
  assert.equal(isValidMarkupBps(1.5), false);
  assert.equal(isValidMarkupBps("100"), false);
  assert.throws(() => splitPayment({ networkCost: 1n, markupBps: 30_001 }), RangeError);
  assert.throws(() => splitPayment({ networkCost: 1n }), RangeError, "there is no anonymous default");
});

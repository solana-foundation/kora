// src/pipe.ts
function pipe(init, ...fns) {
  return fns.reduce((acc, fn) => fn(acc), init);
}

export { pipe };
//# sourceMappingURL=index.node.mjs.map
//# sourceMappingURL=index.node.mjs.map
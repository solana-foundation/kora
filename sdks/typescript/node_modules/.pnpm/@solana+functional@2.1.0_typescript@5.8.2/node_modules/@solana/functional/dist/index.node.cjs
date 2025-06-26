'use strict';

// src/pipe.ts
function pipe(init, ...fns) {
  return fns.reduce((acc, fn) => fn(acc), init);
}

exports.pipe = pipe;
//# sourceMappingURL=index.node.cjs.map
//# sourceMappingURL=index.node.cjs.map
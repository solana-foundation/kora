'use strict';

// src/parse-json-with-bigints.ts
function parseJsonWithBigInts(json) {
  return JSON.parse(wrapIntegersInBigIntValueObject(json), (_, value) => {
    return isBigIntValueObject(value) ? unwrapBigIntValueObject(value) : value;
  });
}
function wrapIntegersInBigIntValueObject(json) {
  const out = [];
  let inQuote = false;
  for (let ii = 0; ii < json.length; ii++) {
    let isEscaped = false;
    if (json[ii] === "\\") {
      out.push(json[ii++]);
      isEscaped = !isEscaped;
    }
    if (json[ii] === '"') {
      out.push(json[ii]);
      if (!isEscaped) {
        inQuote = !inQuote;
      }
      continue;
    }
    if (!inQuote) {
      const consumedNumber = consumeNumber(json, ii);
      if (consumedNumber?.length) {
        ii += consumedNumber.length - 1;
        if (consumedNumber.match(/\.|[eE]-/)) {
          out.push(consumedNumber);
        } else {
          out.push(wrapBigIntValueObject(consumedNumber));
        }
        continue;
      }
    }
    out.push(json[ii]);
  }
  return out.join("");
}
function consumeNumber(json, ii) {
  const JSON_NUMBER_REGEX = /^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/;
  if (!json[ii]?.match(/[-\d]/)) {
    return null;
  }
  const numberMatch = json.slice(ii).match(JSON_NUMBER_REGEX);
  return numberMatch ? numberMatch[0] : null;
}
function wrapBigIntValueObject(value) {
  return `{"$n":"${value}"}`;
}
function unwrapBigIntValueObject({ $n }) {
  if ($n.match(/[eE]/)) {
    const [units, exponent] = $n.split(/[eE]/);
    return BigInt(units) * BigInt(10) ** BigInt(exponent);
  }
  return BigInt($n);
}
function isBigIntValueObject(value) {
  return !!value && typeof value === "object" && "$n" in value && typeof value.$n === "string";
}

// src/rpc-message.ts
var _nextMessageId = 0n;
function getNextMessageId() {
  const id = _nextMessageId;
  _nextMessageId++;
  return id.toString();
}
function createRpcMessage(request) {
  return {
    id: getNextMessageId(),
    jsonrpc: "2.0",
    method: request.methodName,
    params: request.params
  };
}

// src/stringify-json-with-bigints.ts
function stringifyJsonWithBigints(value, space) {
  return unwrapBigIntValueObject2(
    JSON.stringify(value, (_, v) => typeof v === "bigint" ? wrapBigIntValueObject2(v) : v, space)
  );
}
function wrapBigIntValueObject2(value) {
  return { $n: `${value}` };
}
function unwrapBigIntValueObject2(value) {
  return value.replace(/\{\s*"\$n"\s*:\s*"(-?\d+)"\s*\}/g, "$1");
}

exports.createRpcMessage = createRpcMessage;
exports.parseJsonWithBigInts = parseJsonWithBigInts;
exports.stringifyJsonWithBigints = stringifyJsonWithBigints;
//# sourceMappingURL=index.browser.cjs.map
//# sourceMappingURL=index.browser.cjs.map
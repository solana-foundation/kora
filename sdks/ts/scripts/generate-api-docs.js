#!/usr/bin/env node

/**
 * Generate JSON-RPC API documentation from TypeScript SDK
 * This script reads the generated TypeDoc output and creates
 * properly formatted JSON-RPC API documentation for Fumadocs
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/**
 * Parse the TypeDoc README to extract method information and response types
 */
function parseTypeDocReadme() {
  const readmePath = path.join(__dirname, '../docs/README.md');
  
  if (!fs.existsSync(readmePath)) {
    console.error('❌ TypeDoc README not found. Run "pnpm run docs" first.');
    process.exit(1);
  }

  const content = fs.readFileSync(readmePath, 'utf-8');
  const methods = {};
  const responseTypes = {};

  // Parse response type interfaces
  const interfaceSections = content.split(/^### /m).slice(1);
  interfaceSections.forEach(section => {
    const nameMatch = section.match(/^(\w+)/);
    if (!nameMatch) return;
    
    const interfaceName = nameMatch[1];
    
    // Extract properties table
    const propsMatch = section.match(/\| Property \| Type \| Description \|\n\| ------ \| ------ \| ------ \|\n([\s\S]*?)(?:\n\n|\*\*\*|###)/);
    if (propsMatch) {
      const properties = {};
      const propLines = propsMatch[1].trim().split('\n');
      propLines.forEach(line => {
        const parts = line.split('|').map(p => p.trim()).filter(p => p);
        if (parts.length >= 3) {
          // Extract property name (remove HTML anchors and backticks)
          const propName = parts[0].replace(/<a id="[^"]*"><\/a>\s*/, '').replace(/`/g, '').replace(/\?$/, '');
          const rawType = parts[1].replace(/`/g, '').replace(/\s*\\\s*/g, '').trim();
          const description = parts[2];
          
          // Keep brackets for array detection, but clean escaped ones
          const type = rawType.replace(/\\</g, '<').replace(/\\>/g, '>').replace(/\\\\/g, '\\');
          
          
          // Map TypeScript types to example values
          let exampleValue;
          
          // Handle array types first
          if (type.includes('string[]') || type.includes('string\\[\\]')) {
            if (propName.includes('token') || propName.includes('payer')) {
              exampleValue = ["3Z1Ef7YaxK8oUMoi6exf7wYZjZKWJJsrzJXSt1c3qrDE"];
            } else if (propName.includes('program')) {
              exampleValue = ["11111111111111111111111111111111"];
            } else if (propName.includes('account')) {
              exampleValue = ["AccountPublicKey1", "AccountPublicKey2"];
            } else {
              exampleValue = ["exampleValue"];
            }
          } else if (type.includes('[]')) {
            // Generic array type
            exampleValue = [];
          } else if (type.includes('string')) {
            if (propName.includes('signature')) exampleValue = "base58Signature";
            else if (propName.includes('transaction')) exampleValue = "base64EncodedTransaction";
            else if (propName.includes('blockhash')) exampleValue = "base58Blockhash";
            else if (propName.includes('address') || propName.includes('pubkey')) exampleValue = "3Z1Ef7YaxK8oUMoi6exf7wYZjZKWJJsrzJXSt1c3qrDE";
            else exampleValue = "exampleValue";
          } else if (type.includes('number')) {
            if (propName.includes('lamports')) exampleValue = 5000;
            else if (propName.includes('token')) exampleValue = 1000000;
            else exampleValue = 0;
          } else if (type.includes('boolean')) {
            exampleValue = true;
          } else {
            // Complex object types
            exampleValue = {};
          }
          
          properties[propName] = exampleValue;
        }
      });
      responseTypes[interfaceName] = properties;
    }
  });

  // Parse method sections
  const methodSections = content.split(/^##### /m).slice(1);
  
  methodSections.forEach(section => {
    // Get method name
    const nameMatch = section.match(/^(\w+)\(\)/);
    if (!nameMatch) return;
    
    const methodName = nameMatch[1];
    
    // Skip constructor and private methods
    if (methodName === 'Constructor' || methodName.startsWith('_')) return;
    
    // Extract description (first paragraph after method signature)
    const descMatch = section.match(/```ts[\s\S]*?```\s*\n\n(.*?)(?:\n\n|#####|\n\s*#####)/);
    const description = descMatch ? descMatch[1].trim() : '';
    
    // Extract example if exists
    const exampleMatch = section.match(/```typescript\n([\s\S]*?)```/);
    const example = exampleMatch ? exampleMatch[1] : '';
    
    // Extract return type
    const returnMatch = section.match(/`Promise`\\<\[`(\w+)`\]/);
    const returnType = returnMatch ? returnMatch[1] : null;
    
    // Extract parameters
    const paramsMatch = section.match(/\| Parameter \| Type \| Description \|\n\| ------ \| ------ \| ------ \|\n([\s\S]*?)(?:\n\n|#####)/);
    const params = {};
    if (paramsMatch) {
      const paramLines = paramsMatch[1].trim().split('\n');
      paramLines.forEach(line => {
        const parts = line.split('|').map(p => p.trim()).filter(p => p);
        if (parts.length >= 3) {
          const paramName = parts[0].replace(/`/g, '');
          params[paramName] = {
            type: parts[1].replace(/[\[\]`\\<>]/g, ''),
            description: parts[2]
          };
        }
      });
    }

    methods[methodName] = {
      description,
      example,
      params,
      returnType,
      responseStructure: returnType ? responseTypes[returnType] : null
    };
  });

  return { methods, responseTypes };
}

/**
 * Generate JSON-RPC examples based on method name and params
 */
function generateJsonRpcExamples(methodName, methodInfo) {
  // Map SDK method names to JSON-RPC method names
  const rpcMethodName = methodName;
  
  // Build params object from methodInfo.params
  const params = {};
  Object.entries(methodInfo.params || {}).forEach(([key, value]) => {
    if (key === 'request') {
      // This is a request object - we need to expand its properties
      // For now, use smart defaults based on method name
      if (methodName === 'estimateTransactionFee') {
        params.transaction = "base64EncodedTransaction";
        params.fee_token = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
      } else if (methodName === 'transferTransaction') {
        params.amount = 1000000;
        params.token = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        params.source = "sourcePublicKey";
        params.destination = "destinationPublicKey";
      } else if (methodName.includes('sign')) {
        params.transaction = "base64EncodedTransaction";
      }
    } else {
      // Direct parameter
      if (value.type.includes('string')) {
        params[key] = key.includes('transaction') ? "base64EncodedTransaction" : "...";
      } else if (value.type.includes('number')) {
        params[key] = 0;
      } else if (value.type.includes('boolean')) {
        params[key] = false;
      }
    }
  });

  // For methods with no params, use empty array
  const hasParams = Object.keys(params).length > 0;
  
  const request = {
    jsonrpc: "2.0",
    id: 1,
    method: rpcMethodName,
    params: hasParams ? params : []
  };

  // Use actual response structure from TypeDoc if available
  const result = methodInfo.responseStructure || {};

  const response = {
    jsonrpc: "2.0",
    id: 1,
    result
  };

  return { request, response };
}

/**
 * Generate method documentation file
 */
function generateMethodDoc(methodName, methodInfo) {
  const fileName = methodName.replace(/([A-Z])/g, '-$1').toLowerCase().replace(/^-/, '');
  
  // Client-side only methods (no actual JSON-RPC calls)
  const clientSideOnly = ['getPaymentInstruction'];
  
  if (clientSideOnly.includes(methodName)) {
    // Extract TypeScript example from docs
    const tsExample = methodInfo.example || `const result = await client.${methodName}({
  // See SDK documentation for parameters
});`;

    return `---
title: ${methodName}
description: ${methodInfo.description || 'TODO: Add description'}
---

<Callout type="info">
  **Client-Side Only Method**: This method is only available in the TypeScript SDK and does not make actual JSON-RPC calls to the server. It constructs payment instructions locally.
</Callout>

## TypeScript SDK Usage

\`\`\`typescript
${tsExample}
\`\`\`

## Response Structure

The method returns a payment instruction object with the following structure:

\`\`\`typescript
${JSON.stringify(methodInfo.responseStructure || {}, null, 2)}
\`\`\`
`;
  }

  // Regular RPC methods
  const { request, response } = generateJsonRpcExamples(methodName, methodInfo);

  // Extract TypeScript example from docs
  const tsExample = methodInfo.example || `const result = await client.${methodName}({
  // See SDK documentation for parameters
});`;

  return `---
title: ${methodName}
description: ${methodInfo.description || 'TODO: Add description'}
---

## JSON-RPC Request

\`\`\`json
${JSON.stringify(request, null, 2)}
\`\`\`

## JSON-RPC Response

\`\`\`json
${JSON.stringify(response, null, 2)}
\`\`\`

## cURL Example

\`\`\`bash
curl -X POST http://localhost:8080 \\
  -H "Content-Type: application/json" \\
  -d '${JSON.stringify(request)}'
\`\`\`

## TypeScript SDK

\`\`\`typescript
${tsExample}
\`\`\`
`;
}

/**
 * Main function to generate API docs
 */
function generateAPIDocs() {
  console.log('📖 Reading TypeDoc output...');
  const { methods, responseTypes } = parseTypeDocReadme();
  console.log(`📋 Found ${Object.keys(methods).length} methods and ${Object.keys(responseTypes).length} response types`);
  
  const outputDir = path.join(__dirname, '../../../docs/api-reference-generated');
  const methodsDir = path.join(outputDir, 'methods');

  // Create directories
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }
  if (!fs.existsSync(methodsDir)) {
    fs.mkdirSync(methodsDir, { recursive: true });
  }

  // Filter to only RPC methods (not utility methods)
  const rpcMethods = [
    'estimateTransactionFee',
    'getBlockhash',
    'getConfig',
    'getPayerSigner',
    'getPaymentInstruction',
    'getSupportedTokens',
    'signAndSendTransaction',
    'signTransaction',
    'transferTransaction'
  ];

  // Generate method documentation
  rpcMethods.forEach(methodName => {
    if (methods[methodName]) {
      const fileName = `${methodName.replace(/([A-Z])/g, '-$1').toLowerCase().replace(/^-/, '')}.mdx`;
      const filePath = path.join(methodsDir, fileName);
      const content = generateMethodDoc(methodName, methods[methodName]);
      
      fs.writeFileSync(filePath, content);
      console.log(`✓ Generated ${fileName}`);
    } else {
      console.warn(`⚠️  Method ${methodName} not found in TypeDoc output`);
    }
  });

  // Generate overview
  const overviewContent = `---
title: JSON-RPC API Overview
description: Kora implements a JSON-RPC 2.0 interface for gasless transaction processing on Solana.
---


## Protocol

- **Standard**: JSON-RPC 2.0
- **Transport**: HTTP POST
- **Content-Type**: application/json
- **Endpoint**: \`http://your-kora-instance/\`

## Available Methods

| Method | Description |
|--------|-------------|
${rpcMethods.map(m => {
  const info = methods[m] || {};
  const fileName = m.replace(/([A-Z])/g, '-$1').toLowerCase().replace(/^-/, '');
  return `| [${m}](./methods/${fileName}) | ${info.description || 'TODO: Add description'} |`;
}).join('\n')}

## Request Format

All requests follow the JSON-RPC 2.0 standard:

\`\`\`json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "methodName",
  "params": {}
}
\`\`\`

## Response Format

Successful responses:

\`\`\`json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {}
}
\`\`\`

Error responses:

\`\`\`json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32600,
    "message": "Invalid request"
  }
}
\`\`\`
`;

  fs.writeFileSync(path.join(outputDir, 'overview.mdx'), overviewContent);
  console.log('✓ Generated overview.mdx');

  console.log('\n✅ API documentation generated successfully!');
  console.log(`📁 Output: ${outputDir}`);
}

// Run the generator
generateAPIDocs();
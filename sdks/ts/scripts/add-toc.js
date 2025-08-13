#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'fs';
import { resolve } from 'path';

const docsPath = resolve('docs/README.md');
const content = readFileSync(docsPath, 'utf8');

// Extract only method names (lines that end with parentheses)
const methodNames = [];
const lines = content.split('\n');

for (const line of lines) {
  const match = line.match(/^#{5}\s+(.+\(\))$/); // Match ##### method()
  if (match) {
    const methodName = match[1];
    const anchor = methodName
      .toLowerCase()
      .replace(/[^a-z0-9\s-]/g, '')
      .replace(/\s+/g, '-');
    
    methodNames.push({ name: methodName, anchor });
  }
}

// Generate TOC with only methods
const toc = ['## Methods', ''];
for (const method of methodNames) {
  toc.push(`- [${method.name}](#${method.anchor})`);
}

// Insert TOC in KoraClient section, before Constructors
const constructorsLineIndex = lines.findIndex(line => line === '#### Constructors');
const newLines = [
  ...lines.slice(0, constructorsLineIndex),
  ...toc,
  '',
  ...lines.slice(constructorsLineIndex)
];

writeFileSync(docsPath, newLines.join('\n'));
console.log('✅ Added methods table of contents to documentation');
#!/usr/bin/env node

const fs = require('fs').promises;
const path = require('path');
const toml = require('@iarna/toml');

function sanitizeText(value) {
  if (typeof value !== 'string') {
    return value;
  }

  return value
    .replace(/\r\n?/g, '\n')
    .replace(/[ \t]+$/gm, '')
    .replace(/[ \t]+(?=\n)/g, '')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

function normalizeErrorCode(value) {
  if (typeof value === 'number') {
    return Number.isSafeInteger(value) ? value : String(value);
  }

  if (typeof value === 'string') {
    const trimmed = value.trim();
    if (/^-?\d+$/.test(trimmed)) {
      const parsed = BigInt(trimmed);
      return parsed > BigInt(Number.MAX_SAFE_INTEGER) || parsed < BigInt(-Number.MAX_SAFE_INTEGER)
        ? trimmed
        : Number(parsed);
    }

    return trimmed;
  }

  if (typeof value === 'bigint') {
    return value.toString();
  }

  return value;
}

function sanitizeAndNormalize(value, key = '') {
  if (Array.isArray(value)) {
    return value.map((item) => sanitizeAndNormalize(item, key));
  }

  if (value && typeof value === 'object') {
    return Object.entries(value).reduce((acc, [entryKey, entryValue]) => {
      const nextKey = key ? `${key}.${entryKey}` : entryKey;
      let normalizedValue = entryValue;

      if (entryKey === 'code' || entryKey === 'error_code') {
        normalizedValue = normalizeErrorCode(entryValue);
      } else if (typeof entryValue === 'string' && /description|summary|explanation|name|severity|category|difficulty|likelihood|source|id/i.test(entryKey)) {
        normalizedValue = sanitizeText(entryValue);
      } else if (entryValue && typeof entryValue === 'object') {
        normalizedValue = sanitizeAndNormalize(entryValue, nextKey);
      }

      acc[entryKey] = normalizedValue;
      return acc;
    }, {});
  }

  if (typeof value === 'string' && /description|summary|explanation|name|severity|category|difficulty|likelihood|source|id/i.test(key)) {
    return sanitizeText(value);
  }

  return value;
}

async function buildTaxonomyJson({ inputPath, outputPath }) {
  const input = await fs.readFile(inputPath, 'utf8');
  const parsed = toml.parse(input);
  const sanitized = sanitizeAndNormalize(parsed);
  const serialized = JSON.stringify(sanitized, null, 2) + '\n';
  await fs.writeFile(outputPath, serialized, 'utf8');
  return serialized;
}

async function main() {
  const workspaceRoot = path.resolve(__dirname, '..', '..', '..');
  const inputPath = path.join(workspaceRoot, 'crates', 'core', 'src', 'taxonomy', 'data', 'contract.toml');
  const outputPath = path.join(__dirname, 'taxonomy.json');

  const serialized = await buildTaxonomyJson({ inputPath, outputPath });
  console.log(`Wrote sanitized taxonomy JSON to ${path.relative(workspaceRoot, outputPath)}`);
  return serialized;
}

if (require.main === module) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}

module.exports = {
  sanitizeText,
  normalizeErrorCode,
  sanitizeAndNormalize,
  buildTaxonomyJson,
};

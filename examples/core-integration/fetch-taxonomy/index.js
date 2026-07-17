#!/usr/bin/env node

const { readFile, writeFile } = require('node:fs/promises');
const path = require('node:path');
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
  let input;
  try {
    input = await readFile(inputPath, 'utf8');
  } catch (error) {
    throw new Error(formatReadError(error, inputPath));
  }

  let parsed;
  try {
    parsed = toml.parse(input);
  } catch (error) {
    throw new Error(`Parse Error: The TOML file exists but contains invalid syntax:\n  ${error.message}`);
  }

  const sanitized = sanitizeAndNormalize(parsed);
  const serialized = JSON.stringify(sanitized, null, 2) + '\n';

  try {
    await writeFile(outputPath, serialized, 'utf8');
  } catch (error) {
    throw new Error(
      `Unexpected file system error while writing the taxonomy JSON file:\n` +
        `  Code: ${error.code || 'unknown'}\n` +
        `  Message: ${error.message}`
    );
  }

  return serialized;
}

function formatReadError(error, inputPath) {
  if (error.code === 'ENOENT') {
    return (
      `Critical Error: The core taxonomy TOML file could not be located at the expected path:\n` +
      `  ${inputPath}\n\n` +
      `Please ensure the crates/core submodule is initialized:\n` +
      `  git submodule update --init --recursive`
    );
  }

  if (error.code === 'EACCES') {
    return (
      `Permission Error: Cannot read the taxonomy TOML file due to a permission restriction:\n` +
      `  ${inputPath}\n\n` +
      `Verify file permissions or check if another process has locked the file.`
    );
  }

  return (
    `Unexpected file system error while reading the taxonomy TOML file:\n` +
    `  Code: ${error.code || 'unknown'}\n` +
    `  Message: ${error.message}`
  );
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
  formatReadError,
};

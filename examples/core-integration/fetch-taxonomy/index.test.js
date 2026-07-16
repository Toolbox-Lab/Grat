const test = require('node:test');
const assert = require('node:assert/strict');
const { sanitizeText, normalizeErrorCode } = require('./index');

test('sanitizeText trims and normalizes pesky whitespace', () => {
  const value = '  Contract error\r\nwith trailing spaces   \r\n';
  assert.equal(sanitizeText(value), 'Contract error\nwith trailing spaces');
});

test('normalizeErrorCode preserves unsafe integers without losing precision', () => {
  assert.equal(normalizeErrorCode(42), 42);
  assert.equal(normalizeErrorCode('9007199254740993'), '9007199254740993');
  assert.equal(normalizeErrorCode(BigInt('9007199254740993')), '9007199254740993');
});

const { readFile } = require('node:fs/promises');
const path = require('node:path');
const TOML = require('@iarna/toml');

const TOML_PATH = path.resolve(
  __dirname,
  '../../../crates/core/src/taxonomy/data/contract.toml'
);

async function fetchTaxonomy() {
  let raw;
  try {
    raw = await readFile(TOML_PATH, { encoding: 'utf-8' });
  } catch (err) {
    if (err.code === 'ENOENT') {
      console.error(
        `Critical Error: The core taxonomy TOML file could not be located at the expected path:\n` +
        `  ${TOML_PATH}\n\n` +
        `Please ensure the crates/core submodule is initialized:\n` +
        `  git submodule update --init --recursive`
      );
      process.exitCode = 1;
      return;
    }
    if (err.code === 'EACCES') {
      console.error(
        `Permission Error: Cannot read the taxonomy TOML file due to a permission restriction:\n` +
        `  ${TOML_PATH}\n\n` +
        `Verify file permissions or check if another process has locked the file.`
      );
      process.exitCode = 1;
      return;
    }
    console.error(
      `Unexpected file system error while reading the taxonomy TOML file:\n` +
      `  Code: ${err.code || 'unknown'}\n` +
      `  Message: ${err.message}`
    );
    process.exitCode = 1;
    return;
  }

  let taxonomy;
  try {
    taxonomy = TOML.parse(raw);
  } catch (err) {
    console.error(
      `Parse Error: The TOML file exists but contains invalid syntax:\n` +
      `  ${err.message}`
    );
    process.exitCode = 1;
    return;
  }

  console.log(JSON.stringify(taxonomy, null, 2));
}

fetchTaxonomy();

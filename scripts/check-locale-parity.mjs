#!/usr/bin/env node
//
// Locale key-parity gate (deep-read finding L9).
//
// Loads every locale JSON under `src/i18n/locales`, flattens each file into a
// set of dot-separated leaf-key paths, and fails (exit 1) if any key exists in
// one locale but is missing from another. This guards against translation
// drift where a key is added to one language file but forgotten in the others.
//
// Usage:
//   node scripts/check-locale-parity.mjs
//
// NOTE: this only checks key *presence* parity across locales, not value
// quality. It never translates or fills missing keys — it just reports the
// asymmetry so a human can reconcile it.

import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const localesDir = path.join(root, "src/i18n/locales");

const localeFiles = fs
  .readdirSync(localesDir)
  .filter((file) => file.endsWith(".json"))
  .sort();

if (localeFiles.length < 2) {
  console.error(
    `Need at least 2 locale files in ${localesDir}, found ${localeFiles.length}`,
  );
  process.exit(2);
}

// Flatten a nested object into a set of dot-separated leaf-key paths.
// Arrays and primitives are treated as leaf values (i18next message values),
// so only plain objects are recursed into.
function flatten(value, prefix, out) {
  for (const [key, child] of Object.entries(value)) {
    const full = prefix ? `${prefix}.${key}` : key;
    if (child && typeof child === "object" && !Array.isArray(child)) {
      flatten(child, full, out);
    } else {
      out.add(full);
    }
  }
  return out;
}

const locales = localeFiles.map((file) => {
  const name = path.basename(file, ".json");
  const raw = fs.readFileSync(path.join(localesDir, file), "utf8");
  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch (err) {
    console.error(`Failed to parse ${file}: ${err.message}`);
    process.exit(2);
  }
  return { name, file, keys: flatten(parsed, "", new Set()) };
});

// Union of every key seen across all locales.
const allKeys = new Set();
for (const locale of locales) {
  for (const key of locale.keys) allKeys.add(key);
}

// For each locale, collect keys present somewhere but missing from it.
const drift = [];
for (const locale of locales) {
  const missing = [];
  for (const key of allKeys) {
    if (!locale.keys.has(key)) missing.push(key);
  }
  if (missing.length > 0) {
    missing.sort();
    drift.push({ locale: locale.name, missing });
  }
}

console.log(
  JSON.stringify(
    {
      locales: locales.map((locale) => ({
        name: locale.name,
        keys: locale.keys.size,
      })),
      totalUniqueKeys: allKeys.size,
      inParity: drift.length === 0,
    },
    null,
    2,
  ),
);

if (drift.length > 0) {
  console.error("\nLocale key parity drift detected:");
  for (const { locale, missing } of drift) {
    console.error(`\n[${locale}.json] missing ${missing.length} key(s):`);
    for (const key of missing) {
      console.error(`  ${key}`);
    }
  }
  process.exit(1);
}

console.log("\nAll locales are in key parity.");

# rust-ngx-translate-lint

A Rust port of [ngx-translate-lint](https://github.com/svoboda-rabstvo/ngx-translate-lint) for improved performance.

## Installation

```bash
npm install -g rust-ngx-translate-lint
```

**Requirements:** [Rust toolchain](https://rustup.rs/) must be installed on your system.

## Usage

### Command Line

```bash
rust-ngx-translate-lint --project "./src/app/**/*.{html,ts}" --languages "./src/assets/i18n/*.json"
```

Or use the short form:

```bash
rust-ngx-translate-lint -p "./src/app/**/*.{html,ts}" -l "./src/assets/i18n/*.json"
```

### Configuration File

You can also use a configuration file (JSON or JS):

```bash
rust-ngx-translate-lint --config .ngx-translate-lint.json
```

**Example .ngx-translate-lint.json:**

```json
{
  "rules": {
    "keysOnViews": "error",
    "zombieKeys": "warning",
    "misprintKeys": "warning",
    "deepSearch": "enable",
    "emptyKeys": "warning",
    "maxWarning": "6",
    "misprintCoefficient": "0.9",
    "ignoredKeys": [
      "library.targetSystem.*",
      "engineering.signal_configurations.behavior.*",
      "languages.*"
    ],
    "ignoredMisprintKeys": [
      "common.button.*",
      "validation.messages.*"
    ]
  },
  "project": "./src/app/**/*.{html,ts}",
  "languages": "./src/assets/i18n/*.json"
}
```

**Example config.js:**

```javascript
module.exports = {
  rules: {
    keysOnViews: 'error',
    zombieKeys: 'warning',
    misprintKeys: 'warning',
    deepSearch: 'enable',
    emptyKeys: 'warning',
    maxWarning: '6',
    misprintCoefficient: '0.9',
    ignoredKeys: [
      'library.targetSystem.*',
      'engineering.signal_configurations.behavior.*',
      'languages.*'
    ],
    ignoredMisprintKeys: [
      'common.button.*',
      'validation.messages.*'
    ]
  },
  project: './src/app/**/*.{html,ts}',
  languages: './src/assets/i18n/*.json'
};
```

### Options

- `-p, --project` - Path to view files (HTML/TS), supports glob patterns (e.g., `"./src/app/**/*.{html,ts}"`)
- `-l, --languages` - Path to translation files (JSON), supports glob patterns (e.g., `"./src/assets/i18n/*.json"`)
- `--config` - Path to configuration file (JSON or JS)
- `--fix-zombies-keys` - Remove unused translation keys
- `--max-warning` - Maximum allowed warnings before exiting with error code

### Configuration File Options

- `project` - Path to view files (same as `-p` option)
- `languages` - Path to translation files (same as `-l` option)
- `ignore` - Array of paths to ignore
- `fixZombiesKeys` - Boolean to enable zombie key removal
- `rules.keysOnViews` - "error" | "warning" | "disable" - Check for missing translations
- `rules.zombieKeys` - "error" | "warning" | "disable" - Check for unused translations
- `rules.emptyKeys` - "error" | "warning" | "disable" - Check for empty values
- `rules.misprintKeys` - "error" | "warning" | "disable" - Check for potential typos
- `rules.deepSearch` - "enable" | "disable" - Enable deep search mode
- `rules.maxWarning` - Maximum warnings allowed
- `rules.misprintCoefficient` - Threshold for misprint detection (0.0-1.0, default: 0.9)
- `rules.ignoredKeys` - Array of key patterns to ignore (supports wildcards like `"prefix.*"`)
- `rules.ignoredMisprintKeys` - Array of key patterns to ignore for misprint detection
- `rules.customRegExpToFindKeys` - Array of custom regex patterns for key extraction

## Features

- ✅ Detects missing translation keys
- ✅ Finds unused translation keys (zombies)
- ✅ Identifies potential typos in translation keys
- ✅ Supports wildcard patterns for ignored keys
- ✅ Supports glob patterns for file paths
- ✅ Configuration file support (JSON/JS)
- ✅ Compatible with original ngx-translate-lint config format
- ✅ Fast performance thanks to Rust
- ✅ Deep search mode for thorough key detection

## npm Integration

Add to your `package.json`:

```json
{
  "scripts": {
    "lint:translations": "rust-ngx-translate-lint --config .ngx-translate-lint.json"
  },
  "devDependencies": {
    "rust-ngx-translate-lint": "^0.1.4"
  }
}
```

Then run:

```bash
npm run lint:translations
```

## Examples

### Basic Usage

```bash
rust-ngx-translate-lint --project "./src/app/**/*.{html,ts}" --languages "./src/assets/i18n/*.json"
```

### With Configuration File

```bash
rust-ngx-translate-lint --config .ngx-translate-lint.json
```

### Remove Zombie Keys

```bash
rust-ngx-translate-lint --config .ngx-translate-lint.json --fix-zombies-keys
```

### In npm Scripts

```bash
npm run lint:translations
```

## License

MIT

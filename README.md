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
ngx-translate-lint -p <project_path> -l <languages_path> -v <views_path>
```

### Configuration File

You can also use a configuration file (JSON or JS):

```bash
ngx-translate-lint -c config.json
```

**Example config.json:**

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

- `-p, --project` - Path to the project
- `-l, --languages` - Path to directory with translation files (JSON), supports glob patterns
- `-v, --views` - Path to directory with view files (HTML/TS), supports glob patterns
- `-c, --config` - Path to configuration file (JSON or JS)
- `-z, --zombies` - Remove unused translation keys
- `-m, --misprintCoefficient` - Threshold for detecting potential misprints (default: 0.9)
- `--maxWarning` - Maximum allowed warnings before exiting with error code
- `--ignoreKeys` - Comma-separated list of keys to ignore (supports wildcards like `prefix.*`)
- `--ignoreMisprintKeys` - Comma-separated list of keys to ignore for misprint detection
- `--deepSearch` - Enable deep search mode (slower but more thorough)
- `--customRegex` - Custom regex pattern for key extraction

## Features

-  Detects missing translation keys
-  Finds unused translation keys (zombies)
-  Identifies potential typos in translation keys
-  Supports wildcard patterns for ignored keys
-  Supports glob patterns for file paths
-  Configuration file support (JSON/JS)
-  Fast performance thanks to Rust

## Examples

### Basic Usage

```bash
ngx-translate-lint -p ./ -l ./src/assets/i18n -v ./src/app
```

### With Glob Patterns

```bash
ngx-translate-lint -l "./src/assets/i18n/*.json" -v "./src/app/**/*.{html,ts}"
```

### With Configuration File

```bash
ngx-translate-lint -c ngx-translate-lint.config.json
```

### Remove Zombie Keys

```bash
ngx-translate-lint -p ./ -l ./src/assets/i18n -v ./src/app -z
```

### Deep Search Mode

```bash
ngx-translate-lint -p ./ -l ./src/assets/i18n -v ./src/app --deepSearch
```

### Ignore Specific Keys

```bash
ngx-translate-lint -l "./src/assets/i18n/*.json" -v "./src/app/**/*.ts" --ignoreKeys "library.*,engineering.signal.*"
```

## License

MIT

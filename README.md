# rust-ngx-translate-lint

A Rust port of [ngx-translate-lint](https://github.com/svoboda-rabstvo/ngx-translate-lint) for improved performance.

## Installation

```bash
npm install -g rust-ngx-translate-lint
```

**Requirements:** [Rust toolchain](https://rustup.rs/) must be installed on your system.

## Usage

```bash
ngx-translate-lint -p <project_path> -l <languages_path> -v <views_path>
```

### Options

- `-p, --project` - Path to the project
- `-l, --languages` - Path to directory with translation files (JSON)
- `-v, --views` - Path to directory with view files (HTML/TS)
- `-z, --zombies` - Remove unused translation keys
- `-m, --misprintCoefficient` - Threshold for detecting potential misprints (default: 0.9)
- `--maxWarning` - Maximum allowed warnings before exiting with error code
- `--ignoreKeys` - Comma-separated list of keys to ignore
- `--ignoreMisprintKeys` - Comma-separated list of keys to ignore for misprint detection
- `--deepSearch` - Enable deep search mode (slower but more thorough)
- `--customRegex` - Custom regex pattern for key extraction

## Features

- ✅ Detects missing translation keys
- ✅ Finds unused translation keys (zombies)
- ✅ Identifies potential typos in translation keys
- ✅ Supports custom regex patterns
- ✅ Fast performance thanks to Rust

## License

MIT

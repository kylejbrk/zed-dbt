# zed-dbt

dbt language support for [Zed](https://zed.dev), powered by [dbt-language-server](https://github.com/j-clemons/dbt-language-server).

## Features

- **Code Completion** — autocomplete for model references, sources, seeds, macros, variables, and functions
- **Hover Information** — inline documentation for dbt resources and SQL functions
- **Go to Definition** — jump to the definition of models, sources, seeds, and macros
- **Go to Schema** — navigate to schema YAML definitions
- **Diagnostics** — optional static analysis via dbt Fusion

## Installation

Install this extension from the Zed extension marketplace by searching for **dbt**.

The extension will automatically download the latest `dbt-language-server` binary from GitHub releases. Alternatively, if you have `dbt-language-server` already installed and available on your `PATH`, the extension will use that instead.

## Requirements

- [Zed](https://zed.dev) editor
- A dbt project with a `dbt_project.yml` at the root

## How It Works

This extension attaches the dbt language server to **SQL** files. When you open a SQL file inside a dbt project (one containing a `dbt_project.yml`), the language server activates and provides completions, hover info, go-to-definition, and more for dbt-specific constructs like `{{ ref('...') }}`, `{{ source('...', '...') }}`, and macros.

## Configuration

No additional configuration is needed for basic usage. By default, the extension uses the community `dbt-language-server` from the worktree `PATH` when available, or downloads and manages the latest release otherwise. This default does not enable Fusion.

You can override the command in your Zed settings:

```json
{
  "lsp": {
    "dbt-language-server": {
      "binary": {
        "path": "./tools/dbt-language-server",
        "arguments": ["--fusion"],
        "env": {
          "DBT_PROFILES_DIR": "/absolute/path/to/profiles"
        }
      }
    }
  }
}
```

- A bare `binary.path`, such as `"dbt-language-server"`, is resolved from the worktree `PATH` and reports an error if it is absent.
- On macOS and Linux, absolute paths beginning with `/` are used unchanged. On Windows, drive-qualified paths and UNC paths beginning with `\\` or `//` are accepted; WASI-style paths such as `/C:/tools/dbt-language-server.exe` are normalized to `C:/tools/dbt-language-server.exe`.
- Relative paths with directory components are resolved against the worktree root, not the extension directory. On Windows, drive-relative paths such as `C:tools\dbt-language-server` and root-relative paths such as `\tools\dbt-language-server` or `/tools/dbt-language-server` are rejected; use a drive-qualified or UNC absolute path instead.
- The command inherits the worktree shell environment. Values in `binary.env` override matching inherited variables. Environment variable names are matched case-insensitively on Windows and case-sensitively on macOS and Linux; duplicate configured Windows case variants such as `Path` and `PATH` are rejected.
- You can omit `path` and configure only `arguments` or `env`; the normal worktree-`PATH`-first, managed-download fallback will still be used.

The extension also forwards `initialization_options` and `settings` from the `dbt-language-server` LSP configuration.

Managed downloads are supported for macOS arm64, macOS amd64, and Linux amd64, matching the assets currently published by `dbt-language-server`. On Linux arm64, Windows, or x86, install `dbt-language-server` on the worktree `PATH` or configure `binary.path`.

### dbt Fusion (optional)

The default remains the community language server without Fusion diagnostics. If you have [dbt Fusion](https://github.com/j-clemons/dbt-language-server#dbt-fusion-static-analysis) installed, enable its static analysis by adding `--fusion` while retaining automatic language-server discovery:

```json
{
  "lsp": {
    "dbt-language-server": {
      "binary": {
        "arguments": ["--fusion"]
      }
    }
  }
}
```

This extension only supplies Zed settings to `dbt-language-server`; it does not change the server's project or profile discovery. Nested-project profile discovery and Fusion execution from the detected dbt project root require a `dbt-language-server` release containing [upstream PR #16](https://github.com/j-clemons/dbt-language-server/pull/16).

## License

MIT

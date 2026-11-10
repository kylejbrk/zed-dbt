# zed-dbt

dbt language support for [Zed](https://zed.dev), powered by the official
`dbt lsp` language server that ships inside the
[dbt CLI](https://docs.getdbt.com/docs/fusion/install-fusion-cli) as of dbt v2.

> **Upgrading from 0.1.x?** This version talks to the official `dbt lsp` server
> instead of downloading a standalone `dbt-language-server` binary. You now need
> dbt v2 or later on your `PATH` — see [Requirements](#requirements). The
> language server ID is unchanged (`dbt-language-server`), so existing
> `lsp` and `language_servers` settings keep working.

## Features

The extension is deliberately thin: Zed owns the editor UX and `dbt lsp` owns
all language intelligence. The server provides:

- **Diagnostics** — compiler and linter feedback from dbt's static analysis
- **Code Completion** — refs, sources, seeds, macros, variables, and functions
- **Hover Information** — documentation for dbt resources and SQL functions
- **Go to Definition / References / Rename** — across models, sources, seeds,
  and macros, including renaming the underlying `.sql` and `.csv` files
- **Signature Help** and **Inlay Hints**
- **Code Lens** and **Semantic Tokens**
- **Formatting** and **Code Actions** (fix-all and lint-only fixes)

## Requirements

- [Zed](https://zed.dev) editor
- [dbt v2 or later](https://docs.getdbt.com/docs/fusion/install-fusion-cli),
  available as `dbt` on your `PATH`. The `dbt lsp` subcommand does not exist in
  dbt v1 / `dbt-core`, so this extension requires v2.
- A dbt project with a `dbt_project.yml` at the root, opened as the Zed worktree

## Installation

Install this extension from the Zed extension marketplace by searching for
**dbt**.

The extension finds `dbt` on the worktree `PATH` and launches it as `dbt lsp`.
Unlike 0.1.x, no language server binary is downloaded — the server is part of
the dbt CLI you already have installed.

## How It Works

The language server attaches to **SQL**, **YAML**, and **Markdown** files. When
you open one inside a dbt project, the server activates and provides
intelligence for dbt constructs such as `{{ ref('...') }}`,
`{{ source('...', '...') }}`, and macros.

Open the dbt project directory as the Zed worktree. The server discovers the
project from Zed's workspace during initialization; when Zed supplies a file
path as the workspace root, the server walks upward to the nearest
`dbt_project.yml`. Profiles and credentials come from the worktree shell
environment and the normal dbt configuration locations.

The extension sets `DBT_LSP_USE_TARGET_LSP=1`, which keeps the server's compile
artifacts in `target/.lsp` so background analysis never clobbers the output of a
`dbt run` in the same project.

## Configuration

No configuration is required. The linter and formatter are **disabled by
default** and are opted into through Zed's project settings:

```json
{
  "lsp": {
    "dbt-language-server": {
      "settings": {
        "lsp": {
          "maxErrorReporting": 100,
          "linter": { "enabled": true },
          "formatter": { "enabled": true }
        }
      }
    }
  }
}
```

The server reads these from a top-level `dbt` configuration section, so the
extension nests whatever you put under `settings` into `dbt` for you. Writing
the `dbt` wrapper yourself also works if you prefer to be explicit:

```json
{ "settings": { "dbt": { "lsp": { "formatter": { "enabled": true } } } } }
```

Supported `dbt.lsp` keys:

| Key                  | Default | Description                                     |
| -------------------- | ------- | ----------------------------------------------- |
| `maxErrorReporting`  | `100`   | Maximum number of diagnostics reported          |
| `linter.enabled`     | `false` | Enable linter diagnostics                       |
| `formatter.enabled`  | `false` | Enable the formatter                            |

### Enabling formatting and code lenses

To format on save, point the language to the server as its formatter:

```json
{
  "code_lens": "on",
  "languages": {
    "SQL": {
      "formatter": "language_server",
      "format_on_save": "on",
      "semantic_tokens": "combined"
    }
  }
}
```

Zed disables code lenses by default. Keep the **root-level** `code_lens`
setting set to `"on"` to see the code lenses `dbt lsp` provides — Zed does not
support `code_lens` as a language-specific override.

### Using a specific dbt binary

Set `binary.path` to launch a particular `dbt`:

```json
{
  "lsp": {
    "dbt-language-server": {
      "binary": {
        "path": "/absolute/path/to/dbt",
        "arguments": ["lsp"]
      }
    }
  }
}
```

`arguments` defaults to `["lsp"]`. Setting it replaces that default outright, so
it must still start with `lsp`; add dbt flags after it, for example
`["lsp", "--profile", "prod", "--target", "dev"]`. `binary.env` adds or
overrides environment variables when a project needs extra credentials.

## Development

Build the extension for Zed's WASI target:

```sh
cargo build --target wasm32-wasip2
```

Run the unit tests on the host target:

```sh
cargo test
```

In Zed, use **Extensions → Install Dev Extension** and select this directory.
Use **View → Toggle Dev Console** or `zed: open log` to inspect extension and
language server startup.

## License

MIT

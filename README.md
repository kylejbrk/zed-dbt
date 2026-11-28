# zed-dbt

dbt language support for [Zed](https://zed.dev), powered by
[dbt-language-server](https://github.com/j-clemons/dbt-language-server).

## Features

- **Code completion** — autocomplete for model references, sources, seeds, macros, variables, and functions
- **Hover information** — inline documentation for dbt resources and SQL functions
- **Go to definition** — jump to models, sources, seeds, and macros
- **Go to schema** — navigate to schema YAML definitions
- **Diagnostics** — optional static analysis via dbt Fusion

## Installation

Install this extension from the Zed extension marketplace by searching for **dbt**.

The extension uses `dbt-language-server` from your worktree `PATH` when available. Otherwise it downloads the latest release and caches it automatically.

## Requirements

- [Zed](https://zed.dev)
- A dbt project with a `dbt_project.yml` at the worktree root

## How it works

The extension attaches the dbt language server to **SQL** and **YAML** files in a dbt project and provides completion, hover information, go-to-definition, and more for dbt constructs such as `{{ ref('...') }}`, `{{ source('...', '...') }}`, and macros.

## Configuration

No additional configuration is needed for basic usage.

### dbt Fusion diagnostics (optional)

If you have [dbt Fusion](https://github.com/j-clemons/dbt-language-server#dbt-fusion-static-analysis) installed, you can enable static analysis diagnostics. See the dbt-language-server documentation for details.

## Optional: official dbt LSP

The extension can also run the official `dbt lsp` server from dbt v2. It is off by default and uses a separate ID, `dbt-lsp`. It requires dbt v2 or later; dbt Core v1 does not include `dbt lsp`.

Installing dbt v2 does not switch servers on its own. The Go server stays the default until you opt in below.

To opt in, add this to `.zed/settings.json`:

```json
{
  "languages": {
    "SQL": { "language_servers": ["dbt-lsp", "!dbt-language-server", "..."] },
    "YAML": { "language_servers": ["dbt-lsp", "!dbt-language-server", "..."] },
    "Markdown": { "language_servers": ["dbt-lsp", "..."] }
  }
}
```

This disables the default Go server for SQL and YAML so the two servers do not run together. If `dbt` v2 is on your `PATH`, no further configuration is needed. On Zed versions without `opt_in_languages` support, exclude `!dbt-lsp` in each language's list to keep it off.

To use a specific binary, set its path and the `lsp` argument (and the target-directory env if you want the same isolation the default PATH setup gets):

```json
{
  "lsp": {
    "dbt-lsp": {
      "binary": {
        "path": "/absolute/path/to/dbt",
        "arguments": ["lsp"],
        "env": { "DBT_LSP_USE_TARGET_LSP": "1" }
      }
    }
  }
}
```

The extension sets `DBT_LSP_USE_TARGET_LSP=1` for the default PATH setup so the server writes under `target/.lsp`. When you set `binary.path`, Zed runs that binary directly, so include the `arguments` and `env` settings above to keep the same behavior.

## Tasks (optional)

[`examples/tasks.jsonc`](examples/tasks.jsonc) has Zed tasks for dbt projects. Copy it to
`<project>/.zed/tasks.json` (or `~/.config/zed/tasks.json` for every project) and run `task: spawn`:

- lineage of the current model (upstream and downstream models, seeds, snapshots, sources and exposures)
- format the current file or the project (`dbt format`)
- compile the current model and print its SQL
- open the compiled file from `target/` in a tab (this needs the `zed` command: Zed → Install CLI)
- preview the current model (`dbt show`) in a pager
- run, test and build the current model

They run `dbt` from your `PATH`; replace `dbt` in the file to use another binary. Model lineage, compiled
SQL, compile, the preview and run/test/build work with dbt v1 and v2 (on v1 the preview shows at most six
columns, and compile prints the SQL twice). Format needs dbt v2 and fails on v1. `run`, `build` and
the preview run queries on your warehouse.

## License

MIT

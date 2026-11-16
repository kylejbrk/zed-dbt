# zed-dbt

dbt language support for [Zed](https://zed.dev), powered by
[dbt-language-server](https://github.com/j-clemons/dbt-language-server) by
default, with opt-in support for the official `dbt lsp` server.

## Installation and default server

Install this extension from the Zed extension marketplace by searching for
**dbt**. Open a dbt project containing `dbt_project.yml` as your Zed worktree.

The existing Go server, `dbt-language-server`, remains the default for **SQL**
and **YAML**. The extension uses it from your worktree `PATH` when available;
otherwise it downloads the latest release and caches it automatically.
Existing users do not need dbt v2 or any settings changes.

The default server provides completion, hover information, go to definition,
and go to schema. For optional Fusion diagnostics, see the
[dbt-language-server documentation](https://github.com/j-clemons/dbt-language-server#dbt-fusion-static-analysis).

## Opting into the official dbt server

The official server has a separate ID, **`dbt-lsp`**, and is disabled by default
for **SQL**, **YAML**, and **Markdown** using Zed's
[`opt_in_languages`](https://zed.dev/docs/extensions/languages#opt-in-language-servers)
manifest setting. It requires [dbt v2 or later](https://docs.getdbt.com/docs/fusion/install-fusion-cli)
and a project compatible with that version. dbt Core v1 does not include
`dbt lsp`.

Use a Zed version that supports `opt_in_languages`. Older versions ignore this
manifest setting and may start both servers; update Zed, or explicitly exclude
`!dbt-lsp` in each language's `language_servers` list to keep using the default.

To select the official server, add this to your project's `.zed/settings.json`:

```json
{
  "languages": {
    "SQL": { "language_servers": ["dbt-lsp", "!dbt-language-server", "..."] },
    "YAML": { "language_servers": ["dbt-lsp", "!dbt-language-server", "..."] },
    "Markdown": { "language_servers": ["dbt-lsp", "..."] }
  },
  "lsp": {
    "dbt-lsp": {
      "binary": {
        "path": "/absolute/path/to/dbt"
      }
    }
  }
}
```

`binary.path` is optional when dbt v2 is available as `dbt` on the worktree
`PATH`. The extension launches it as `dbt lsp`; no official server binary is
downloaded. Server selection is per language, so you can opt in only for the
languages you need. The example disables the Go server to avoid duplicate
results while preserving other servers through `"..."`.

Profiles and credentials come from the worktree shell environment and normal
dbt configuration locations. The extension sets `DBT_LSP_USE_TARGET_LSP=1` for
the official server to keep background compile artifacts in `target/.lsp`.

### Official server features

The official `dbt lsp` server provides:

- **Diagnostics** — compiler and linter feedback from dbt's static analysis
- **Code Completion** — refs, sources, seeds, macros, variables, and functions
- **Hover Information** — documentation for dbt resources and SQL functions
- **Go to Definition / References / Rename** — across models, sources, seeds,
  and macros, including renaming the underlying `.sql` and `.csv` files
- **Signature Help** and **Inlay Hints**
- **Code Lens** and **Semantic Tokens**
- **Formatting** and **Code Actions** (fix-all and lint-only fixes)

### Official server settings

Settings for the two servers are independent. Keep existing Go server settings
under `lsp.dbt-language-server`; configure the official server under
`lsp.dbt-lsp`. The official linter and formatter are disabled by default:

```json
{
  "lsp": {
    "dbt-lsp": {
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

### Official server formatting and code lenses

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
    "dbt-lsp": {
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

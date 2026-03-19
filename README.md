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

No additional configuration is needed for basic usage. The extension downloads and manages the language server binary automatically.

### dbt Fusion (optional)

If you have [dbt Fusion](https://github.com/j-clemons/dbt-language-server#dbt-fusion-static-analysis) installed, you can enable static analysis diagnostics. Refer to the dbt-language-server documentation for details.

## License

MIT
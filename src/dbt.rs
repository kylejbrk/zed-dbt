use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result, Worktree};

/// The language server ID declared in `extension.toml`. Kept stable across the
/// move to the official server so existing `languages.*.language_servers` and
/// `lsp.*` entries in user settings keep working.
const LANGUAGE_SERVER_ID: &str = "dbt-language-server";

/// The dbt CLI. The official language server is the `lsp` subcommand of this
/// binary rather than a separately distributed executable, and it exists only
/// in dbt v2 and later.
const DBT_COMMAND: &str = "dbt";

/// Subcommand that starts the official language server on stdio.
const LSP_SUBCOMMAND: &str = "lsp";

/// Redirects the server's compile artifacts to `target/.lsp` instead of
/// `target/`, so background analysis never clobbers the output of a `dbt run`
/// in the same project. The first-party VS Code extension sets this too.
const USE_TARGET_LSP_ENV: &str = "DBT_LSP_USE_TARGET_LSP";

struct DbtExtension;

impl DbtExtension {
    fn settings(worktree: &Worktree) -> Result<LspSettings> {
        LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree)
    }

    /// Resolves the `dbt` executable: an explicit `binary.path` wins, otherwise
    /// it is looked up on the worktree `PATH`.
    fn command_path(worktree: &Worktree, settings: &LspSettings) -> Result<String> {
        settings
            .binary
            .as_ref()
            .and_then(|binary| binary.path.as_deref())
            .filter(|path| !path.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| worktree.which(DBT_COMMAND))
            .ok_or_else(|| {
                format!(
                    "Could not find the `{DBT_COMMAND}` executable on the worktree PATH. \
                     Install dbt v2 or later \
                     (https://docs.getdbt.com/docs/fusion/install-fusion-cli), \
                     or set `lsp.{LANGUAGE_SERVER_ID}.binary.path` in your Zed settings to an \
                     absolute path."
                )
            })
    }

    /// Defaults to `dbt lsp`. An explicit `binary.arguments` replaces the
    /// default outright, so extra dbt flags such as `--profile` or `--target`
    /// are passed as `["lsp", "--profile", "prod"]`.
    fn command_args(settings: &LspSettings) -> Vec<String> {
        settings
            .binary
            .as_ref()
            .and_then(|binary| binary.arguments.clone())
            .filter(|arguments| !arguments.is_empty())
            .unwrap_or_else(|| vec![LSP_SUBCOMMAND.to_string()])
    }

    fn command_env(worktree: &Worktree, settings: &LspSettings) -> Vec<(String, String)> {
        let mut env = worktree.shell_env();
        set_env(&mut env, USE_TARGET_LSP_ENV, "1");

        // An explicit `binary.env` wins over the inherited shell environment,
        // for projects that need extra credentials or configuration.
        if let Some(binary) = &settings.binary {
            if let Some(extra_env) = &binary.env {
                for (key, value) in extra_env {
                    set_env(&mut env, key, value);
                }
            }
        }

        env
    }

    /// The server asks for the top-level `dbt` configuration section and reads
    /// its own settings from `dbt.lsp`. Zed answers `workspace/configuration`
    /// out of `settings.lsp.<id>.settings`, so lift the ergonomic Zed shape
    /// (`settings` holding `lsp`) into the `dbt` section the server expects,
    /// while leaving an already-wrapped object untouched.
    fn workspace_configuration(settings: &LspSettings) -> Option<zed::serde_json::Value> {
        let settings = settings.settings.clone()?;

        if settings.get("dbt").is_some() {
            Some(settings)
        } else {
            Some(zed::serde_json::json!({ "dbt": settings }))
        }
    }
}

impl zed::Extension for DbtExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        if language_server_id.as_ref() != LANGUAGE_SERVER_ID {
            return Err(format!("Unsupported language server: {language_server_id}"));
        }

        let settings = Self::settings(worktree)?;
        Ok(zed::Command {
            command: Self::command_path(worktree, &settings)?,
            args: Self::command_args(&settings),
            env: Self::command_env(worktree, &settings),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        if language_server_id.as_ref() != LANGUAGE_SERVER_ID {
            return Ok(None);
        }

        Ok(Self::settings(worktree)?.initialization_options)
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        if language_server_id.as_ref() != LANGUAGE_SERVER_ID {
            return Ok(None);
        }

        Ok(Self::workspace_configuration(&Self::settings(worktree)?))
    }
}

/// Sets `key` to `value`, replacing an existing entry rather than appending a
/// duplicate. Duplicate keys in a `Command.env` list are resolved
/// inconsistently across platforms, so the last writer has to win here.
fn set_env(env: &mut Vec<(String, String)>, key: &str, value: &str) {
    if let Some((_, existing_value)) = env.iter_mut().find(|(existing_key, _)| existing_key == key)
    {
        *existing_value = value.to_string();
    } else {
        env.push((key.to_string(), value.to_string()));
    }
}

zed::register_extension!(DbtExtension);

#[cfg(test)]
mod tests {
    use super::*;
    use zed_extension_api::settings::CommandSettings;

    fn with_arguments(arguments: Vec<&str>) -> LspSettings {
        LspSettings {
            binary: Some(CommandSettings {
                path: None,
                arguments: Some(arguments.into_iter().map(str::to_string).collect()),
                env: None,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn set_env_replaces_existing_values_without_duplicates() {
        let mut env = vec![("PATH".to_string(), "old".to_string())];
        set_env(&mut env, "PATH", "new");
        set_env(&mut env, USE_TARGET_LSP_ENV, "1");

        assert_eq!(
            env,
            vec![
                ("PATH".to_string(), "new".to_string()),
                (USE_TARGET_LSP_ENV.to_string(), "1".to_string()),
            ]
        );
    }

    #[test]
    fn defaults_to_the_lsp_subcommand() {
        assert_eq!(DbtExtension::command_args(&LspSettings::default()), ["lsp"]);
    }

    #[test]
    fn explicit_arguments_replace_the_default_subcommand() {
        let settings = with_arguments(vec!["lsp", "--profile", "prod"]);

        assert_eq!(
            DbtExtension::command_args(&settings),
            ["lsp", "--profile", "prod"]
        );
    }

    #[test]
    fn empty_arguments_fall_back_to_the_default_subcommand() {
        assert_eq!(DbtExtension::command_args(&with_arguments(vec![])), ["lsp"]);
    }

    #[test]
    fn wraps_lsp_settings_in_the_dbt_workspace_configuration_section() {
        let settings = LspSettings {
            settings: Some(zed::serde_json::json!({
                "lsp": {
                    "linter": {"enabled": true},
                    "maxErrorReporting": 100,
                }
            })),
            ..Default::default()
        };

        assert_eq!(
            DbtExtension::workspace_configuration(&settings),
            Some(zed::serde_json::json!({
                "dbt": {
                    "lsp": {
                        "linter": {"enabled": true},
                        "maxErrorReporting": 100,
                    }
                }
            }))
        );
    }

    #[test]
    fn preserves_an_explicit_dbt_workspace_configuration_section() {
        let settings = LspSettings {
            settings: Some(zed::serde_json::json!({
                "dbt": {"lsp": {"formatter": {"enabled": true}}}
            })),
            ..Default::default()
        };

        assert_eq!(
            DbtExtension::workspace_configuration(&settings),
            settings.settings
        );
    }

    #[test]
    fn absent_settings_send_no_workspace_configuration() {
        assert_eq!(
            DbtExtension::workspace_configuration(&LspSettings::default()),
            None
        );
    }
}

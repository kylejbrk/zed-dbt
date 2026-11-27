use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result, Worktree};

/// Separate IDs keep existing Go server settings and defaults intact.
const GO_LANGUAGE_SERVER_ID: &str = "dbt-language-server";
const LANGUAGE_SERVER_ID: &str = "dbt-lsp";

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

struct DbtExtension {
    cached_binary_path: Option<String>,
}

impl DbtExtension {
    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        // First, check if the user has it on their PATH already
        if let Some(path) = worktree.which("dbt-language-server") {
            return Ok(path);
        }

        // If we've already downloaded it, return the cached path
        if let Some(path) = &self.cached_binary_path {
            if std::fs::metadata(path).is_ok_and(|m| m.is_file()) {
                return Ok(path.clone());
            }
        }

        let result = self.download_or_update(language_server_id);
        if let Err(error) = &result {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Failed(error.to_string()),
            );
        }
        result
    }

    fn download_or_update(&mut self, language_server_id: &LanguageServerId) -> Result<String> {
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            "j-clemons/dbt-language-server",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, arch) = zed::current_platform();
        let asset_name = format!(
            "dbt-language-server-{os}-{arch}",
            os = match platform {
                zed::Os::Mac => "darwin",
                zed::Os::Linux => "linux",
                zed::Os::Windows => "windows",
            },
            arch = match arch {
                zed::Architecture::Aarch64 => "arm64",
                zed::Architecture::X8664 => "amd64",
                zed::Architecture::X86 => "amd64",
            },
        );

        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| {
                format!(
                    "no matching release asset found for platform {asset_name}. Available assets: {}",
                    release
                        .assets
                        .iter()
                        .map(|a| a.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;

        // Download directly as a flat file named with the version to avoid
        // needing to create subdirectories (download_file does not create
        // parent directories automatically).
        let binary_path = format!("dbt-language-server-{}", release.version);

        if !std::fs::metadata(&binary_path).is_ok_and(|m| m.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &asset.download_url,
                &binary_path,
                zed::DownloadedFileType::Uncompressed,
            )
            .map_err(|e| format!("failed to download file: {e}"))?;

            zed::make_file_executable(&binary_path)?;

            self.remove_stale_binaries(&binary_path);
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }

    fn remove_stale_binaries(&self, current_binary: &str) {
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("dbt-language-server-") && name != current_binary {
                        let _ = std::fs::remove_file(name);
                    }
                }
            }
        }
    }

    fn settings(worktree: &Worktree) -> Result<LspSettings> {
        LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree)
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
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        if language_server_id.as_ref() == GO_LANGUAGE_SERVER_ID {
            return Ok(zed::Command {
                command: self.language_server_binary_path(language_server_id, worktree)?,
                args: vec![],
                env: vec![],
            });
        }

        if language_server_id.as_ref() != LANGUAGE_SERVER_ID {
            return Err(format!("Unsupported language server: {language_server_id}"));
        }

        // Zed applies binary.path/arguments/env itself; only the PATH fallback and dbt defaults belong here.
        let command = worktree.which(DBT_COMMAND).ok_or_else(|| {
            format!(
                "Could not find the `{DBT_COMMAND}` executable on the worktree PATH. \
                 Install dbt v2 or later \
                 (https://docs.getdbt.com/docs/fusion/install-fusion-cli), \
                 or set `lsp.{LANGUAGE_SERVER_ID}.binary.path` together with \
                 `lsp.{LANGUAGE_SERVER_ID}.binary.arguments` to `[\"{LSP_SUBCOMMAND}\"]` \
                 in your Zed settings."
            )
        })?;

        Ok(zed::Command {
            command,
            args: vec![LSP_SUBCOMMAND.to_string()],
            env: vec![(USE_TARGET_LSP_ENV.to_string(), "1".to_string())],
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

zed::register_extension!(DbtExtension);

#[cfg(test)]
mod tests {
    use super::*;

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

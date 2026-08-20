use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

const LANGUAGE_SERVER_NAME: &str = "dbt-language-server";

struct DbtExtension {
    cached_binary_path: Option<String>,
}

impl DbtExtension {
    fn resolve_configured_binary_path(path: &str, worktree: &zed::Worktree) -> Result<String> {
        if path.contains('/') || path.contains('\\') {
            return Ok(path.to_string());
        }

        worktree.which(path).ok_or_else(|| {
            format!(
                "configured dbt language server executable `{path}` was not found on the worktree PATH"
            )
        })
    }

    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        // First, check if the user has it on their PATH already
        if let Some(path) = worktree.which(LANGUAGE_SERVER_NAME) {
            return Ok(path);
        }

        // If we've already downloaded it, return the cached path
        if let Some(path) = &self.cached_binary_path {
            if std::fs::metadata(path).map_or(false, |m| m.is_file()) {
                return Ok(path.clone());
            }
        }

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

        if !std::fs::metadata(&binary_path).map_or(false, |m| m.is_file()) {
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
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }

    fn make_language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let settings = LspSettings::for_worktree(LANGUAGE_SERVER_NAME, worktree)?;
        let binary_settings = settings.binary.as_ref();

        let args = binary_settings
            .and_then(|binary| binary.arguments.clone())
            .unwrap_or_default();
        let env = binary_settings
            .and_then(|binary| binary.env.as_ref())
            .map(|env| {
                env.iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let command = if let Some(path) = binary_settings.and_then(|binary| binary.path.as_deref())
        {
            Self::resolve_configured_binary_path(path, worktree)?
        } else {
            self.language_server_binary_path(language_server_id, worktree)?
        };

        Ok(zed::Command { command, args, env })
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
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        match self.make_language_server_command(language_server_id, worktree) {
            Ok(command) => {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::None,
                );
                Ok(command)
            }
            Err(error) => {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::Failed(error.clone()),
                );
                Err(error)
            }
        }
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(LspSettings::for_worktree(LANGUAGE_SERVER_NAME, worktree)?.initialization_options)
    }

    fn language_server_workspace_configuration(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(LspSettings::for_worktree(LANGUAGE_SERVER_NAME, worktree)?.settings)
    }
}

zed::register_extension!(DbtExtension);

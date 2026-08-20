use std::collections::HashMap;

use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

const LANGUAGE_SERVER_NAME: &str = "dbt-language-server";
const MANUAL_BINARY_HINT: &str = "Install `dbt-language-server` on the worktree PATH or configure `lsp.dbt-language-server.binary.path`.";

#[derive(Debug, PartialEq, Eq)]
enum ConfiguredPathKind {
    BareExecutable,
    Absolute,
    Relative,
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn is_windows_drive_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    has_windows_drive_prefix(path) && bytes.len() >= 3 && (bytes[2] == b'/' || bytes[2] == b'\\')
}

fn classify_configured_path(path: &str) -> Result<ConfiguredPathKind> {
    if path.is_empty() {
        return Err("configured dbt language server path must not be empty".to_string());
    }

    if has_windows_drive_prefix(path) && !is_windows_drive_absolute(path) {
        return Err(format!(
            "configured dbt language server path `{path}` is drive-relative; use an absolute path such as `C:\\path\\to\\dbt-language-server`"
        ));
    }

    if path.starts_with('\\') && !path.starts_with("\\\\") {
        return Err(format!(
            "configured dbt language server path `{path}` is Windows root-relative; use a drive-qualified absolute path or UNC path"
        ));
    }

    if path.starts_with('/') || path.starts_with("\\\\") || is_windows_drive_absolute(path) {
        Ok(ConfiguredPathKind::Absolute)
    } else if path.contains('/') || path.contains('\\') {
        Ok(ConfiguredPathKind::Relative)
    } else {
        Ok(ConfiguredPathKind::BareExecutable)
    }
}

fn join_worktree_path(root_path: &str, relative_path: &str) -> String {
    let separator = if root_path.contains('\\') { '\\' } else { '/' };
    let root_path = root_path.trim_end_matches(['/', '\\']);

    if root_path.is_empty() {
        format!("{separator}{relative_path}")
    } else {
        format!("{root_path}{separator}{relative_path}")
    }
}

fn merge_environment(
    mut environment: Vec<(String, String)>,
    configured: Option<&HashMap<String, String>>,
) -> Vec<(String, String)> {
    if let Some(configured) = configured {
        for (key, value) in configured {
            environment.retain(|(existing_key, _)| existing_key != key);
            environment.push((key.clone(), value.clone()));
        }
    }

    environment
}

fn managed_asset_name(platform: zed::Os, arch: zed::Architecture) -> Result<&'static str> {
    match (platform, arch) {
        (zed::Os::Mac, zed::Architecture::Aarch64) => {
            Ok("dbt-language-server-darwin-arm64")
        }
        (zed::Os::Mac, zed::Architecture::X8664) => {
            Ok("dbt-language-server-darwin-amd64")
        }
        (zed::Os::Linux, zed::Architecture::X8664) => {
            Ok("dbt-language-server-linux-amd64")
        }
        (zed::Os::Linux, zed::Architecture::Aarch64) => Err(format!(
            "managed dbt-language-server downloads are not available for Linux arm64. {MANUAL_BINARY_HINT}"
        )),
        (zed::Os::Windows, _) => Err(format!(
            "managed dbt-language-server downloads are not available for Windows. {MANUAL_BINARY_HINT}"
        )),
        (_, zed::Architecture::X86) => Err(format!(
            "managed dbt-language-server downloads are not available for x86. {MANUAL_BINARY_HINT}"
        )),
    }
}

struct DbtExtension {
    cached_binary_path: Option<String>,
}

impl DbtExtension {
    fn resolve_configured_binary_path(path: &str, worktree: &zed::Worktree) -> Result<String> {
        match classify_configured_path(path)? {
            ConfiguredPathKind::BareExecutable => worktree.which(path).ok_or_else(|| {
                format!(
                    "configured dbt language server executable `{path}` was not found on the worktree PATH"
                )
            }),
            ConfiguredPathKind::Absolute => Ok(path.to_string()),
            ConfiguredPathKind::Relative => {
                Ok(join_worktree_path(&worktree.root_path(), path))
            }
        }
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

        let (platform, arch) = zed::current_platform();
        let asset_name = managed_asset_name(platform, arch)?;

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
        let env = merge_environment(
            worktree.shell_env(),
            binary_settings.and_then(|binary| binary.env.as_ref()),
        );

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_configured_paths_without_host_path_semantics() {
        assert_eq!(
            classify_configured_path("dbt-language-server").unwrap(),
            ConfiguredPathKind::BareExecutable
        );
        assert_eq!(
            classify_configured_path("./bin/dbt-language-server").unwrap(),
            ConfiguredPathKind::Relative
        );
        assert_eq!(
            classify_configured_path("/opt/dbt-language-server").unwrap(),
            ConfiguredPathKind::Absolute
        );
        assert_eq!(
            classify_configured_path(r"C:\tools\dbt-language-server").unwrap(),
            ConfiguredPathKind::Absolute
        );
        assert_eq!(
            classify_configured_path(r"\\server\tools\dbt-language-server").unwrap(),
            ConfiguredPathKind::Absolute
        );
        assert!(classify_configured_path(r"C:tools\dbt-language-server").is_err());
        assert!(classify_configured_path(r"\tools\dbt-language-server").is_err());
    }

    #[test]
    fn resolves_relative_paths_against_posix_and_windows_worktree_roots() {
        assert_eq!(
            join_worktree_path("/workspace/project", "bin/dbt-language-server"),
            "/workspace/project/bin/dbt-language-server"
        );
        assert_eq!(
            join_worktree_path(r"C:\workspace\project", r"bin\dbt-language-server.exe"),
            r"C:\workspace\project\bin\dbt-language-server.exe"
        );
    }

    #[test]
    fn merges_configured_environment_over_worktree_environment() {
        let worktree_environment = vec![
            ("PATH".to_string(), "/worktree/bin".to_string()),
            ("HOME".to_string(), "/home/user".to_string()),
        ];
        let configured = HashMap::from([
            ("PATH".to_string(), "/configured/bin".to_string()),
            ("DBT_PROFILES_DIR".to_string(), "profiles".to_string()),
        ]);

        let environment = merge_environment(worktree_environment, Some(&configured));

        assert!(environment.contains(&("PATH".to_string(), "/configured/bin".to_string())));
        assert!(environment.contains(&("HOME".to_string(), "/home/user".to_string())));
        assert!(environment.contains(&("DBT_PROFILES_DIR".to_string(), "profiles".to_string())));
        assert_eq!(
            environment.iter().filter(|(key, _)| key == "PATH").count(),
            1
        );
    }

    #[test]
    fn selects_only_published_managed_assets() {
        assert_eq!(
            managed_asset_name(zed::Os::Mac, zed::Architecture::Aarch64).unwrap(),
            "dbt-language-server-darwin-arm64"
        );
        assert_eq!(
            managed_asset_name(zed::Os::Mac, zed::Architecture::X8664).unwrap(),
            "dbt-language-server-darwin-amd64"
        );
        assert_eq!(
            managed_asset_name(zed::Os::Linux, zed::Architecture::X8664).unwrap(),
            "dbt-language-server-linux-amd64"
        );
        assert!(
            managed_asset_name(zed::Os::Linux, zed::Architecture::Aarch64)
                .unwrap_err()
                .contains("Linux arm64")
        );
        assert!(
            managed_asset_name(zed::Os::Windows, zed::Architecture::X8664)
                .unwrap_err()
                .contains("Windows")
        );
        assert!(managed_asset_name(zed::Os::Mac, zed::Architecture::X86)
            .unwrap_err()
            .contains("x86"));
    }
}

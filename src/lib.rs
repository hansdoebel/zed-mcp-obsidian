use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use zed_extension_api::{
    self as zed, settings::ContextServerSettings, ContextServerId, Project, Result,
};

const MCP_SDK_PACKAGE: &str = "@modelcontextprotocol/sdk";
const MCP_SDK_VERSION: &str = "1.29.0";
const PROXY_JS: &str = include_str!("../server/proxy.js");
const DISPATCH_JS: &str = include_str!("../server/dispatch.js");
const TOOLS_JS: &str = include_str!("../server/tools.js");

#[derive(Debug, Deserialize, JsonSchema)]
struct ObsidianMcpSettings {
    /// Name of the default vault. Its tools appear unprefixed.
    /// Must be a key in `vaults`.
    default_vault: String,
    /// Map of vault name to per-vault settings. At least one entry required.
    /// Vault names must match [A-Za-z0-9_-]+ and must not contain `__`.
    vaults: BTreeMap<String, VaultSettings>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct VaultSettings {
    /// API key for this vault's Local REST API plugin.
    api_key: String,
    /// Host (default: 127.0.0.1).
    #[serde(default)]
    host: Option<String>,
    /// Port (default: 27124).
    #[serde(default)]
    port: Option<String>,
    /// Protocol: "http" or "https" (default: https).
    #[serde(default)]
    protocol: Option<String>,
}

#[derive(Debug, Serialize)]
struct ProxyConfig {
    default_vault: String,
    vaults: BTreeMap<String, ProxyVaultConfig>,
}

#[derive(Debug, Serialize)]
struct ProxyVaultConfig {
    url: String,
    api_key: String,
}

struct ObsidianMcpExtension {
    did_install_dependencies: bool,
}

impl zed::Extension for ObsidianMcpExtension {
    fn new() -> Self {
        Self {
            did_install_dependencies: false,
        }
    }

    fn context_server_command(
        &mut self,
        context_server_id: &ContextServerId,
        project: &Project,
    ) -> Result<zed::Command> {
        self.install_npm_dependencies()?;
        self.ensure_server_files()?;

        let settings = Self::settings(context_server_id, project)?;
        let proxy_config = Self::build_proxy_config(&settings)?;
        let vaults_json = zed::serde_json::to_string(&proxy_config)
            .map_err(|e| format!("Failed to serialize vault config: {e}"))?;

        let script_path = env::current_dir()
            .map_err(|e| e.to_string())?
            .join("proxy.js")
            .to_string_lossy()
            .to_string();

        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args: vec![script_path],
            env: vec![
                ("OBSIDIAN_VAULTS_JSON".to_string(), vaults_json),
                ("NODE_TLS_REJECT_UNAUTHORIZED".to_string(), "0".to_string()),
            ],
        })
    }

    fn context_server_configuration(
        &mut self,
        _context_server_id: &ContextServerId,
        _project: &Project,
    ) -> Result<Option<zed::ContextServerConfiguration>> {
        let installation_instructions =
            include_str!("../configuration/installation_instructions.md").to_string();
        let default_settings =
            include_str!("../configuration/default_settings.jsonc").to_string();
        let settings_schema = schemars::schema_for!(ObsidianMcpSettings);
        let settings_schema_str =
            zed::serde_json::to_string_pretty(&settings_schema).unwrap_or_default();
        Ok(Some(zed::ContextServerConfiguration {
            installation_instructions,
            default_settings,
            settings_schema: settings_schema_str,
        }))
    }
}

impl ObsidianMcpExtension {
    fn install_npm_dependencies(&mut self) -> Result<()> {
        if self.did_install_dependencies {
            return Ok(());
        }
        let mcp_version = zed::npm_package_installed_version(MCP_SDK_PACKAGE)?;
        if mcp_version.as_deref() != Some(MCP_SDK_VERSION) {
            zed::npm_install_package(MCP_SDK_PACKAGE, MCP_SDK_VERSION)?;
        }
        self.did_install_dependencies = true;
        Ok(())
    }

    fn ensure_server_files(&self) -> Result<()> {
        fs::write("proxy.js", PROXY_JS).map_err(|e| e.to_string())?;
        fs::write("dispatch.js", DISPATCH_JS).map_err(|e| e.to_string())?;
        fs::write("tools.js", TOOLS_JS).map_err(|e| e.to_string())?;
        let package_json =
            r#"{"type":"module","dependencies":{"@modelcontextprotocol/sdk":"*"}}"#;
        fs::write("package.json", package_json).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn settings(
        context_server_id: &ContextServerId,
        project: &Project,
    ) -> Result<ObsidianMcpSettings> {
        let context_settings =
            ContextServerSettings::for_project(context_server_id.as_ref(), project)
                .map_err(|e| format!("Failed to read context server settings: {e}"))?;
        let value = context_settings.settings.ok_or_else(settings_help)?;
        zed::serde_json::from_value(value)
            .map_err(|e| format!("Invalid settings: {e}\n\n{}", settings_help()))
    }

    fn build_proxy_config(s: &ObsidianMcpSettings) -> Result<ProxyConfig> {
        if s.vaults.is_empty() {
            return Err("`vaults` map must not be empty.".to_string());
        }
        if !s.vaults.contains_key(&s.default_vault) {
            return Err(format!(
                "`default_vault` is \"{}\" but is not a key in `vaults`.",
                s.default_vault
            ));
        }
        let mut proxy_vaults = BTreeMap::new();
        for (name, v) in &s.vaults {
            validate_vault_name(name)?;
            let api_key = sanitize_api_key(&v.api_key)
                .map_err(|e| format!("Vault \"{name}\" api_key {e}."))?;
            let host = v.host.as_deref().unwrap_or("127.0.0.1");
            let port = v.port.as_deref().unwrap_or("27124");
            let protocol = v.protocol.as_deref().unwrap_or("https");
            let url = format!("{protocol}://{host}:{port}/mcp/");
            proxy_vaults.insert(
                name.clone(),
                ProxyVaultConfig { url, api_key },
            );
        }
        Ok(ProxyConfig {
            default_vault: s.default_vault.clone(),
            vaults: proxy_vaults,
        })
    }
}

fn sanitize_api_key(raw: &str) -> Result<String, String> {
    let s = raw.trim_start();

    // Strip optional "Bearer" prefix (case-insensitive) when followed by ≥1
    // ASCII whitespace char. as_bytes()[..6] is safe under len() >= 6 because
    // we compare bytes, not str, avoiding UTF-8 boundary panics on inputs that
    // start with multi-byte chars.
    let s = if s.len() >= 6 && s.as_bytes()[..6].eq_ignore_ascii_case(b"Bearer") {
        let after = &s[6..];
        let trimmed = after.trim();
        if trimmed.len() < after.len() {
            trimmed
        } else {
            s.trim_end()
        }
    } else {
        s.trim_end()
    };

    if s.is_empty() {
        return Err("is empty".to_string());
    }
    if s.chars().any(|c| c.is_ascii_whitespace()) {
        return Err("contains internal whitespace".to_string());
    }
    if !s.bytes().all(|b| (0x21..=0x7E).contains(&b)) {
        return Err("contains non-printable or non-ASCII characters".to_string());
    }
    let len = s.len();
    if !(8..=512).contains(&len) {
        return Err(format!(
            "length {len} is outside the accepted range 8..=512"
        ));
    }

    Ok(s.to_string())
}

fn validate_vault_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err("Vault name must not be empty.".to_string());
    }
    if name.contains("__") {
        return Err(format!(
            "Vault name \"{name}\" must not contain `__` (reserved as tool-name prefix separator)."
        ));
    }
    let valid = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !valid {
        return Err(format!(
            "Vault name \"{name}\" contains invalid characters. Only [A-Za-z0-9_-] allowed."
        ));
    }
    Ok(())
}

fn settings_help() -> String {
    "Missing or invalid settings. Add to your Zed settings.json:\n\n\
    \"context_servers\": {\n  \
      \"obsidian-mcp\": {\n    \
        \"settings\": {\n      \
          \"default_vault\": \"personal\",\n      \
          \"vaults\": {\n        \
            \"personal\": { \"api_key\": \"YOUR_KEY\" }\n      \
          }\n    \
        }\n  \
      }\n\
    }"
    .to_string()
}

zed::register_extension!(ObsidianMcpExtension);

#[cfg(test)]
mod tests {
    use super::sanitize_api_key;

    // Canonical 64-char hex sample from the spec.
    const HEX: &str =
        "8f4d21f62b4f5285ae2d0e16eec8dfbe635a95389549590dbc7f9e13276e9f1d";

    #[test]
    fn accepts_bare_hex_key() {
        assert_eq!(sanitize_api_key(HEX).unwrap(), HEX);
    }

    #[test]
    fn strips_bearer_prefix() {
        let raw = format!("Bearer {HEX}");
        assert_eq!(sanitize_api_key(&raw).unwrap(), HEX);
    }

    #[test]
    fn trims_outer_whitespace_around_bearer() {
        let raw = format!("  Bearer {HEX}  ");
        assert_eq!(sanitize_api_key(&raw).unwrap(), HEX);
    }

    #[test]
    fn bearer_prefix_is_case_insensitive_lower() {
        let raw = format!("bearer {HEX}");
        assert_eq!(sanitize_api_key(&raw).unwrap(), HEX);
    }

    #[test]
    fn bearer_prefix_is_case_insensitive_upper() {
        let raw = format!("BEARER {HEX}");
        assert_eq!(sanitize_api_key(&raw).unwrap(), HEX);
    }

    #[test]
    fn collapses_multiple_spaces_after_bearer() {
        let raw = format!("Bearer    {HEX}");
        assert_eq!(sanitize_api_key(&raw).unwrap(), HEX);
    }

    #[test]
    fn accepts_tab_after_bearer() {
        let raw = format!("Bearer\t{HEX}");
        assert_eq!(sanitize_api_key(&raw).unwrap(), HEX);
    }

    #[test]
    fn rejects_empty_string() {
        assert_eq!(sanitize_api_key("").unwrap_err(), "is empty");
    }

    #[test]
    fn rejects_whitespace_only() {
        assert_eq!(sanitize_api_key("   ").unwrap_err(), "is empty");
    }

    #[test]
    fn rejects_bearer_with_no_token() {
        assert_eq!(sanitize_api_key("Bearer ").unwrap_err(), "is empty");
    }

    #[test]
    fn rejects_internal_whitespace() {
        let raw = format!("Bearer {HEX} extra");
        assert_eq!(
            sanitize_api_key(&raw).unwrap_err(),
            "contains internal whitespace"
        );
    }

    #[test]
    fn rejects_embedded_newline() {
        // 8+ chars so length isn't the failure trigger.
        assert_eq!(
            sanitize_api_key("abc\ndefxx").unwrap_err(),
            "contains internal whitespace"
        );
    }

    #[test]
    fn rejects_embedded_null_byte() {
        assert_eq!(
            sanitize_api_key("abc\0defxx").unwrap_err(),
            "contains non-printable or non-ASCII characters"
        );
    }

    #[test]
    fn rejects_non_ascii() {
        assert_eq!(
            sanitize_api_key("abc\u{1F511}defxx").unwrap_err(),
            "contains non-printable or non-ASCII characters"
        );
    }

    #[test]
    fn rejects_too_short() {
        assert_eq!(
            sanitize_api_key("abcdefg").unwrap_err(),
            "length 7 is outside the accepted range 8..=512"
        );
    }

    #[test]
    fn rejects_too_long() {
        let raw = "a".repeat(513);
        assert_eq!(
            sanitize_api_key(&raw).unwrap_err(),
            "length 513 is outside the accepted range 8..=512"
        );
    }

    #[test]
    fn bearer_without_space_is_treated_as_raw_key() {
        // "BearerNoSpaceKey" — no whitespace after "Bearer", so the literal
        // string is the key. 16 chars, all printable ASCII, no internal ws.
        assert_eq!(
            sanitize_api_key("BearerNoSpaceKey").unwrap(),
            "BearerNoSpaceKey"
        );
    }
}

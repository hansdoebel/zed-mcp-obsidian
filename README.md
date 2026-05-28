# Obsidian MCP Server for Zed

A [Zed](https://zed.dev) extension that connects Zed to one or more
[Obsidian](https://obsidian.md) vaults through the MCP (Model Context Protocol)
server built into the
[Local REST API](https://github.com/coddingtonbear/obsidian-local-rest-api)
community plugin.

The extension launches a small local bridge that maintains MCP client
connections to each configured vault (opened lazily on first use) and exposes
their tools to Zed as one combined MCP server.

## Prerequisites

In each Obsidian vault you want to expose:

1. Install and enable the **Local REST API** community plugin
2. Enable the MCP server in **Settings → Local REST API & MCP Server**
3. Copy the API key from the same settings page
4. Set a unique port per vault (e.g. 27124, 27125, 27126) if you'll run more
   than one vault at a time

## Setup

Add to your Zed `settings.json`:

```jsonc
{
  "context_servers": {
    "obsidian-mcp": {
      "settings": {
        "default_vault": "personal",
        "vaults": {
          "personal": {
            "api_key": "YOUR_KEY"
          },
          "work": {
            "api_key": "YOUR_KEY",
            "port": "27125"
          }
        }
      }
    }
  }
}
```

The default vault's tools appear unprefixed (`vault_read`, `search_simple`, …).
Other vaults' tools are prefixed with their name (`work__vault_read`,
`work__search_simple`, …).

### Per-vault optional fields

| Field | Default | Description |
|-------|---------|-------------|
| `host` | `127.0.0.1` | Obsidian host |
| `port` | `27124` | Obsidian port |
| `protocol` | `https` | `http` or `https` |

The extension trusts the plugin's self-signed HTTPS certificate automatically.

Vault names must match `[A-Za-z0-9_-]+` and must not contain `__` (reserved
as the tool-name prefix separator).

### Lazy connections

Connections are opened the first time a vault's tool is called. A vault that's
not open in Obsidian only produces an error when you try to use it — closed
vaults don't block startup. Reopening a vault recovers without restarting Zed.

## Tools

The extension exposes the 15 tools from the Local REST API plugin's MCP server,
once unprefixed (for the default vault) and once per non-default vault with the
`<vaultname>__` prefix.

<details>
<summary>Tools</summary>

| Tool | Description |
|------|-------------|
| `vault_list` | List files and subdirectories inside a vault directory |
| `vault_read` | Read a file's content, frontmatter, tags, and stat |
| `vault_write` | Create or overwrite a vault file |
| `vault_append` | Append content to the end of a vault file |
| `vault_patch` | Patch a heading, block reference, or frontmatter field |
| `vault_delete` | Delete a vault file |
| `vault_get_document_map` | List the headings, blocks, and frontmatter fields in a file |
| `active_file_get_path` | Path of the file currently open in Obsidian |
| `periodic_note_get_path` | Path of the current periodic note |
| `search_query` | Search with a JsonLogic query over note metadata |
| `search_simple` | Full-text search using Obsidian's built-in search |
| `tag_list` | List all tags across the vault with usage counts |
| `command_list` | List all registered Obsidian commands |
| `command_execute` | Execute an Obsidian command by ID |
| `open_file` | Open a file in the Obsidian UI |

</details>

If the plugin adds tools, regenerate `server/tools.js` with the
`server/scripts/dump-tools.mjs` helper and rebuild.

## Upgrading from 0.1.x

The settings shape changed in 0.2.0. Replace the old flat keys
(`obsidian_api_key`, `obsidian_host`, `obsidian_port`, `obsidian_protocol`)
with the new `default_vault` + `vaults` map shown above.

## Example Prompts

- *"List all files in my vault"* (uses the default vault)
- *"Search my work vault for files mentioning 'project alpha'"*
- *"Summarize the last 5 daily notes in my personal vault"*
- *"Create 'Meeting Notes.md' in my work vault"*

## Development

```bash
# Run proxy unit tests
cd server && npm install && npm test

# Build the WASM extension
cargo build --target wasm32-wasip1 --release
cp target/wasm32-wasip1/release/obsidian_mcp.wasm extension.wasm
```

Install as a dev extension in Zed: **Extensions → Install Dev Extension** →
select this directory.

## License

MIT

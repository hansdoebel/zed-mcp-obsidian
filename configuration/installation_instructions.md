# Obsidian MCP Server — Setup

This extension connects Zed to the **MCP server built into the Local REST API
community plugin**. One installation can serve multiple Obsidian vaults — each
vault runs its own Local REST API instance on a different port.

## Prerequisites (per vault)

1. **Install the Obsidian Local REST API plugin**
   - In Obsidian, go to **Settings → Community plugins → Browse**
   - Search for **Local REST API**
   - Install and enable it
   - Plugin page: <https://github.com/coddingtonbear/obsidian-local-rest-api>

2. **Enable the MCP server** in **Settings → Local REST API & MCP Server**

3. **Copy the API key** from the same settings page

4. **Set a unique port** per vault under the same settings page if you plan to
   run more than one vault at a time.

## Configuration

The default vault's tools appear unprefixed (`vault_read`, `search_simple`, …).
Other vaults' tools are prefixed with `<vaultname>__` (e.g. `work__vault_read`).
Connections are opened lazily — a vault only needs to be open in Obsidian when
you actually use one of its tools.

```jsonc
{
  "default_vault": "personal",
  "vaults": {
    "personal": { "api_key": "YOUR_KEY" },
    "work":     { "api_key": "YOUR_KEY", "port": "27125" }
  }
}
```

Vault names must match `[A-Za-z0-9_-]+` and must not contain `__`.

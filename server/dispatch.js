// Pure routing helpers. No I/O, no MCP SDK imports — kept testable in isolation.

/**
 * Split tool name into optional vault prefix + tool name.
 * Returns { vault: string|null, toolName: string }.
 * vault is null when the name has no `__` separator OR when the prefix
 * doesn't match any configured vault (the full name then routes to default).
 */
export function parseToolName(name, vaultNames) {
  const sep = name.indexOf("__");
  if (sep === -1) {
    return { vault: null, toolName: name };
  }
  const candidate = name.slice(0, sep);
  if (vaultNames.has(candidate)) {
    return { vault: candidate, toolName: name.slice(sep + 2) };
  }
  return { vault: null, toolName: name };
}

/**
 * Synthesize the full tool list: default vault's tools unprefixed,
 * other vaults' tools prefixed `<vault>__`. Non-default vaults are sorted
 * alphabetically for stable ordering.
 */
export function buildToolList(baseTools, defaultVault, allVaultNames) {
  const out = [];
  for (const t of baseTools) out.push(t);
  const others = [...allVaultNames]
    .filter((v) => v !== defaultVault)
    .sort();
  for (const vault of others) {
    for (const t of baseTools) {
      out.push({ ...t, name: `${vault}__${t.name}` });
    }
  }
  return out;
}

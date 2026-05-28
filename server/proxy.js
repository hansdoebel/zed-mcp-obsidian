import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import { OBSIDIAN_TOOLS } from "./tools.js";
import { parseToolName, buildToolList } from "./dispatch.js";

// Obsidian's REST API uses a self-signed certificate.
process.env.NODE_TLS_REJECT_UNAUTHORIZED = "0";

const configJson = process.env.OBSIDIAN_VAULTS_JSON;
if (!configJson) {
  console.error("OBSIDIAN_VAULTS_JSON environment variable is required.");
  process.exit(1);
}

let config;
try {
  config = JSON.parse(configJson);
} catch (err) {
  console.error(`Invalid OBSIDIAN_VAULTS_JSON: ${err.message}`);
  process.exit(1);
}

const { default_vault: defaultVault, vaults: vaultConfigs } = config;
const vaultNames = new Set(Object.keys(vaultConfigs));

// Per-vault lazy client state.
const vaultState = new Map();
for (const name of vaultNames) {
  vaultState.set(name, { config: vaultConfigs[name], client: null });
}

async function getClient(vaultName) {
  const state = vaultState.get(vaultName);
  if (!state) {
    throw new Error(`unknown vault "${vaultName}"`);
  }
  if (state.client) return state.client;

  const client = new Client(
    { name: "obsidian-mcp-proxy", version: "0.2.0" },
    { capabilities: {} },
  );
  const transport = new StreamableHTTPClientTransport(new URL(state.config.url), {
    requestInit: {
      headers: { Authorization: `Bearer ${state.config.api_key}` },
    },
  });
  try {
    await client.connect(transport);
  } catch (err) {
    // Do NOT cache failure — let next call retry so user-recoverable issues
    // (Obsidian closed, wrong vault open) clear without restarting Zed.
    const msg = err?.message ?? String(err);
    throw new Error(`vault "${vaultName}" unreachable: ${msg}`);
  }
  state.client = client;
  return client;
}

const server = new Server(
  { name: "obsidian-mcp", version: "0.2.0" },
  { capabilities: { tools: {} } },
);

server.setRequestHandler(ListToolsRequestSchema, async () => ({
  tools: buildToolList(OBSIDIAN_TOOLS, defaultVault, vaultNames),
}));

server.setRequestHandler(CallToolRequestSchema, async (req) => {
  const { name, arguments: args } = req.params;
  const { vault: prefix, toolName } = parseToolName(name, vaultNames);
  const targetVault = prefix ?? defaultVault;

  let client;
  try {
    client = await getClient(targetVault);
  } catch (err) {
    // Surface as a tool-level error so the model sees the message and can react,
    // rather than as a protocol-level JSON-RPC error.
    return {
      content: [{ type: "text", text: err.message }],
      isError: true,
    };
  }

  try {
    return await client.callTool({ name: toolName, arguments: args });
  } catch (err) {
    const msg = err?.message ?? String(err);
    return {
      content: [{ type: "text", text: `vault "${targetVault}" call failed: ${msg}` }],
      isError: true,
    };
  }
});

const transport = new StdioServerTransport();
await server.connect(transport);

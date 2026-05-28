import test from "node:test";
import assert from "node:assert/strict";
import { parseToolName, buildToolList } from "../dispatch.js";

const vaults = new Set(["personal", "work", "research"]);

test("parseToolName: bare name routes to default (no vault)", () => {
  assert.deepEqual(parseToolName("vault_read", vaults), {
    vault: null,
    toolName: "vault_read",
  });
});

test("parseToolName: known prefix is stripped", () => {
  assert.deepEqual(parseToolName("work__vault_read", vaults), {
    vault: "work",
    toolName: "vault_read",
  });
});

test("parseToolName: unknown prefix falls through to default with full name", () => {
  assert.deepEqual(parseToolName("unknown__vault_read", vaults), {
    vault: null,
    toolName: "unknown__vault_read",
  });
});

test("parseToolName: only the first __ is treated as separator", () => {
  // If a future tool itself contained `__`, the bare lookup wins.
  assert.deepEqual(parseToolName("work__some__weird_tool", vaults), {
    vault: "work",
    toolName: "some__weird_tool",
  });
});

test("buildToolList: default vault tools unprefixed, others prefixed", () => {
  const base = [
    { name: "vault_read", description: "read", inputSchema: {} },
    { name: "vault_write", description: "write", inputSchema: {} },
  ];
  const result = buildToolList(base, "personal", vaults);
  const names = result.map((t) => t.name);
  assert.deepEqual(names, [
    "vault_read",
    "vault_write",
    "research__vault_read",
    "research__vault_write",
    "work__vault_read",
    "work__vault_write",
  ]);
});

test("buildToolList: preserves descriptions and schemas on prefixed copies", () => {
  const base = [
    { name: "vault_read", description: "read a file", inputSchema: { type: "object", properties: { path: { type: "string" } } } },
  ];
  const result = buildToolList(base, "personal", new Set(["personal", "work"]));
  const work = result.find((t) => t.name === "work__vault_read");
  assert.equal(work.description, "read a file");
  assert.deepEqual(work.inputSchema, { type: "object", properties: { path: { type: "string" } } });
});

test("buildToolList: single-vault setup produces only unprefixed tools", () => {
  const base = [{ name: "vault_read", description: "", inputSchema: {} }];
  const result = buildToolList(base, "personal", new Set(["personal"]));
  assert.deepEqual(result.map((t) => t.name), ["vault_read"]);
});

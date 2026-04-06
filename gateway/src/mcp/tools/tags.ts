import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { ApiContext } from "../api";
import { callTool, textResult, errorResult } from "../api";
import { tr } from "../i18n";
import { callJsonTool, dedupeIds, validateExclusiveTargets } from "./common";

export function registerTagTools(server: McpServer, api: ApiContext, locale: string): void {
  server.registerTool("list_tags", {
    description: tr("tool-list-tags", locale),
    inputSchema: {
      limit: z.number().int().positive().max(100).optional().describe("Maximum number of tags to return"),
    },
  }, ({ limit }) => {
    const query = limit !== undefined ? `?limit=${limit}` : "";
    return callTool(api, "GET", `/api/tags${query}`);
  });

  server.registerTool("create_tag", {
    description: tr("tool-create-tag", locale),
    inputSchema: {
      name: z.string().describe("Tag name"),
      color: z.string().optional().describe("Tag color (hex, e.g. '#ff0000')"),
      parent_tag_id: z.string().optional().describe("Parent tag ID for hierarchical tags"),
    },
  }, ({ name, color, parent_tag_id }) =>
    callTool(api, "POST", "/api/tags", { name, color, parent_tag_id }));

  server.registerTool("assign_tag", {
    description: tr("tool-assign-tag", locale),
    inputSchema: {
      tag_id: z.string().describe("The tag ID"),
      item_id: z.string().optional().describe("Item ID (provide item_id or list_id)"),
      list_id: z.string().optional().describe("List ID (provide item_id or list_id)"),
    },
  }, async ({ tag_id, item_id, list_id }) => {
    const result = await setTagLinks(api, {
      action: "assign",
      tag_ids: [tag_id],
      item_ids: item_id ? [item_id] : undefined,
      list_ids: list_id ? [list_id] : undefined,
    });
    if (result.isError) return result;
    if (item_id) return textResult(`Tag assigned to item ${item_id}`);
    if (list_id) return textResult(`Tag assigned to list ${list_id}`);
    return errorResult("Provide either item_id or list_id");
  });

  server.registerTool("remove_tag", {
    description: tr("tool-remove-tag", locale),
    inputSchema: {
      tag_id: z.string().describe("The tag ID"),
      item_id: z.string().optional().describe("Item ID (provide item_id or list_id)"),
      list_id: z.string().optional().describe("List ID (provide item_id or list_id)"),
    },
  }, async ({ tag_id, item_id, list_id }) => {
    const result = await setTagLinks(api, {
      action: "remove",
      tag_ids: [tag_id],
      item_ids: item_id ? [item_id] : undefined,
      list_ids: list_id ? [list_id] : undefined,
    });
    if (result.isError) return result;
    if (item_id) return textResult(`Tag removed from item ${item_id}`);
    if (list_id) return textResult(`Tag removed from list ${list_id}`);
    return errorResult("Provide either item_id or list_id");
  });

  server.registerTool("set_tag_links", {
    description: tr("tool-set-tag-links", locale),
    inputSchema: {
      action: z.enum(["assign", "remove"]).describe("Whether to assign or remove links"),
      tag_ids: z.array(z.string()).min(1).describe("Tag IDs to link"),
      item_ids: z.array(z.string()).optional().describe("Target item IDs"),
      list_ids: z.array(z.string()).optional().describe("Target list IDs"),
    },
  }, ({ action, tag_ids, item_ids, list_ids }) =>
    setTagLinks(api, { action, tag_ids, item_ids, list_ids }));

  server.registerTool("get_tag_items", {
    description: tr("tool-get-tagged-items", locale),
    inputSchema: {
      tag_id: z.string().describe("The tag ID"),
      recursive: z.boolean().default(false).describe("Include items from child tags"),
    },
  }, ({ tag_id, recursive }) => {
    const params = `?recursive=${recursive ? "true" : "false"}`;
    return callTool(api, "GET", `/api/tags/${tag_id}/items${params}`);
  });

  async function setTagLinks(
    api: ApiContext,
    input: {
      action: "assign" | "remove";
      tag_ids: string[];
      item_ids?: string[];
      list_ids?: string[];
    }
  ) {
    const tag_ids = dedupeIds(input.tag_ids);
    const item_ids = dedupeIds(input.item_ids);
    const list_ids = dedupeIds(input.list_ids);
    if (tag_ids.length === 0) {
      return errorResult("Provide at least one tag_id.");
    }
    const targetError = validateExclusiveTargets(
      [item_ids, list_ids],
      "Provide item_ids or list_ids.",
      "Provide exactly one target kind: item_ids or list_ids."
    );
    if (targetError) {
      return errorResult(targetError);
    }
    return callJsonTool(api, "PATCH", "/api/tag-links", {
      action: input.action,
      tag_ids,
      item_ids: item_ids.length > 0 ? item_ids : undefined,
      list_ids: list_ids.length > 0 ? list_ids : undefined,
    });
  }
}

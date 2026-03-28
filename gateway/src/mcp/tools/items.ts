import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { ApiContext } from "../api";
import { callTool, ensureFeatures } from "../api";
import { tr } from "../i18n";

export function registerItemTools(server: McpServer, api: ApiContext, locale: string): void {
  server.registerTool("get_list_items", {
    description: tr("tool-get-items", locale),
    inputSchema: {
      list_id: z.string().describe("The list ID"),
    },
  }, ({ list_id }) => callTool(api, "GET", `/api/lists/${list_id}/items`));

  server.registerTool("add_item", {
    description: tr("tool-add-item", locale),
    inputSchema: {
      list_id: z.string().describe("The list ID"),
      title: z.string().describe("Item title"),
      description: z.string().optional().describe("Item description"),
      quantity: z.number().optional().describe("Target quantity"),
      unit: z.string().optional().describe("Unit of measurement"),
      start_date: z.string().optional().describe("Start date YYYY-MM-DD"),
      start_time: z.string().optional().describe("Start time HH:MM"),
      deadline: z.string().optional().describe("Deadline YYYY-MM-DD"),
      deadline_time: z.string().optional().describe("Deadline time HH:MM"),
      hard_deadline: z.string().optional().describe("Hard deadline YYYY-MM-DD"),
    },
  }, async ({ list_id, ...fields }) => {
    await ensureFeatures(api, list_id, fields);
    return callTool(api, "POST", `/api/lists/${list_id}/items`, fields);
  });

  server.registerTool("update_item", {
    description: tr("tool-update-item", locale),
    inputSchema: {
      list_id: z.string().describe("The list ID"),
      item_id: z.string().describe("The item ID"),
      title: z.string().optional().describe("New title"),
      description: z.string().nullable().optional().describe("New description (null to clear)"),
      completed: z.boolean().optional().describe("Completion state"),
      quantity: z.number().optional().describe("Target quantity"),
      actual_quantity: z.number().optional().describe("Actual quantity (auto-completes when >= quantity)"),
      unit: z.string().nullable().optional().describe("Unit (null to clear)"),
      start_date: z.string().nullable().optional().describe("Start date YYYY-MM-DD (null to clear)"),
      start_time: z.string().nullable().optional().describe("Start time HH:MM (null to clear)"),
      deadline: z.string().nullable().optional().describe("Deadline YYYY-MM-DD (null to clear)"),
      deadline_time: z.string().nullable().optional().describe("Deadline time HH:MM (null to clear)"),
      hard_deadline: z.string().nullable().optional().describe("Hard deadline YYYY-MM-DD (null to clear)"),
    },
  }, async ({ list_id, item_id, ...fields }) => {
    await ensureFeatures(api, list_id, fields);
    return callTool(api, "PUT", `/api/lists/${list_id}/items/${item_id}`, fields);
  });

  server.registerTool("toggle_item", {
    description: tr("tool-toggle-item", locale),
    inputSchema: {
      list_id: z.string().describe("The list ID"),
      item_id: z.string().describe("The item ID"),
      completed: z.boolean().describe("New completed state"),
    },
  }, ({ list_id, item_id, completed }) =>
    callTool(api, "PUT", `/api/lists/${list_id}/items/${item_id}`, { completed }));

  server.registerTool("move_item", {
    description: tr("tool-move-item", locale),
    inputSchema: {
      item_id: z.string().describe("The item ID"),
      target_list_id: z.string().describe("Target list ID"),
    },
  }, ({ item_id, target_list_id }) =>
    callTool(api, "PATCH", `/api/items/${item_id}/move`, { list_id: target_list_id }));
}

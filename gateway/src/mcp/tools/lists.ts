import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { ApiContext } from "../api";
import { callTool } from "../api";
import { tr } from "../i18n";

export function registerListTools(server: McpServer, api: ApiContext, locale: string): void {
  server.registerTool("list_lists", {
    description: tr("tool-list-lists", locale),
    inputSchema: {},
  }, () => callTool(api, "GET", "/api/lists"));

  server.registerTool("create_list", {
    description: tr("tool-create-list", locale),
    inputSchema: {
      name: z.string().describe("The name for the new list"),
      list_type: z.enum(["checklist", "zakupy", "pakowanie", "terminarz", "custom"])
        .default("checklist")
        .describe("Type: checklist (default), zakupy (shopping), pakowanie (packing), terminarz (schedule), custom"),
    },
  }, ({ name, list_type }) => callTool(api, "POST", "/api/lists", { name, list_type }));

  server.registerTool("update_list", {
    description: tr("tool-update-list", locale),
    inputSchema: {
      list_id: z.string().describe("The ID of the list to update"),
      name: z.string().optional().describe("New name"),
      description: z.string().nullable().optional().describe("New description (null to clear)"),
      list_type: z.enum(["checklist", "zakupy", "pakowanie", "terminarz", "custom"]).optional().describe("New type"),
      archived: z.boolean().optional().describe("Archive/unarchive"),
    },
  }, ({ list_id, ...fields }) => callTool(api, "PUT", `/api/lists/${list_id}`, fields));

  server.registerTool("move_list_to_container", {
    description: tr("tool-move-list", locale),
    inputSchema: {
      list_id: z.string().describe("The list ID"),
      container_id: z.string().nullable().describe("Container ID (null to remove from container)"),
    },
  }, ({ list_id, container_id }) =>
    callTool(api, "PATCH", `/api/lists/${list_id}/container`, { container_id }));
}

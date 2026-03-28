import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { ApiContext } from "../api";
import { apiCall, callTool, textResult, errorResult } from "../api";

export function registerTagTools(server: McpServer, api: ApiContext): void {
  server.registerTool("list_tags", {
    description: "List all tags for the current user",
    inputSchema: {},
  }, () => callTool(api, "GET", "/api/tags"));

  server.registerTool("create_tag", {
    description: "Create a new tag",
    inputSchema: {
      name: z.string().describe("Tag name"),
      color: z.string().optional().describe("Tag color (hex, e.g. '#ff0000')"),
      parent_tag_id: z.string().optional().describe("Parent tag ID for hierarchical tags"),
    },
  }, ({ name, color, parent_tag_id }) =>
    callTool(api, "POST", "/api/tags", { name, color, parent_tag_id }));

  server.registerTool("assign_tag", {
    description: "Assign a tag to an item or a list",
    inputSchema: {
      tag_id: z.string().describe("The tag ID"),
      item_id: z.string().optional().describe("Item ID (provide item_id or list_id)"),
      list_id: z.string().optional().describe("List ID (provide item_id or list_id)"),
    },
  }, async ({ tag_id, item_id, list_id }) => {
    if (item_id) {
      const res = await apiCall(api, "POST", `/api/items/${item_id}/tags`, { tag_id });
      return res.ok ? textResult(`Tag assigned to item ${item_id}`) : errorResult(`API error ${res.status}: ${await res.text()}`);
    }
    if (list_id) {
      const res = await apiCall(api, "POST", `/api/lists/${list_id}/tags`, { tag_id });
      return res.ok ? textResult(`Tag assigned to list ${list_id}`) : errorResult(`API error ${res.status}: ${await res.text()}`);
    }
    return errorResult("Provide either item_id or list_id");
  });

  server.registerTool("remove_tag", {
    description: "Remove a tag from an item or a list",
    inputSchema: {
      tag_id: z.string().describe("The tag ID"),
      item_id: z.string().optional().describe("Item ID (provide item_id or list_id)"),
      list_id: z.string().optional().describe("List ID (provide item_id or list_id)"),
    },
  }, async ({ tag_id, item_id, list_id }) => {
    if (item_id) {
      const res = await apiCall(api, "DELETE", `/api/items/${item_id}/tags/${tag_id}`);
      return res.ok ? textResult(`Tag removed from item ${item_id}`) : errorResult(`API error ${res.status}: ${await res.text()}`);
    }
    if (list_id) {
      const res = await apiCall(api, "DELETE", `/api/lists/${list_id}/tags/${tag_id}`);
      return res.ok ? textResult(`Tag removed from list ${list_id}`) : errorResult(`API error ${res.status}: ${await res.text()}`);
    }
    return errorResult("Provide either item_id or list_id");
  });

  server.registerTool("get_tag_items", {
    description: "Get all items tagged with a specific tag (optionally including child tags)",
    inputSchema: {
      tag_id: z.string().describe("The tag ID"),
      recursive: z.boolean().default(false).describe("Include items from child tags"),
    },
  }, ({ tag_id, recursive }) => {
    const params = recursive ? "?recursive=true" : "";
    return callTool(api, "GET", `/api/tags/${tag_id}/items${params}`);
  });
}

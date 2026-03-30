import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { ApiContext } from "../api";
import { apiCall, callTool, errorResult, jsonResult } from "../api";
import { tr } from "../i18n";
import { callJsonTool, dedupeIds } from "./common";

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
      parent_list_id: z.string().nullable().optional().describe("Parent list ID for creating a sublist"),
      container_id: z.string().nullable().optional().describe("Container ID for creating a list inside a container"),
    },
  }, ({ name, list_type, parent_list_id, container_id }) =>
    callTool(api, "POST", "/api/lists", { name, list_type, parent_list_id, container_id }));

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
  }, async ({ list_id, container_id }) => {
    const result = await setListPlacement(api, {
      list_ids: [list_id],
      container_id,
    });
    if (result.isError) return result;
    try {
      const parsed = JSON.parse(result.content[0].text) as { moved_lists?: unknown[] };
      return jsonResult(parsed.moved_lists?.[0] ?? null);
    } catch {
      return errorResult("Failed to parse list placement response.");
    }
  });

  server.registerTool("get_list_sublists", {
    description: tr("tool-get-list-sublists", locale),
    inputSchema: {
      list_id: z.string().describe("Parent list ID"),
    },
  }, ({ list_id }) => callTool(api, "GET", `/api/lists/${list_id}/sublists`));

  server.registerTool("set_list_placement", {
    description: tr("tool-set-list-placement", locale),
    inputSchema: {
      list_ids: z.array(z.string()).min(1).describe("List IDs to move"),
      parent_list_id: z.string().nullable().optional().describe("Target parent list ID (for sublists)"),
      container_id: z.string().nullable().optional().describe("Target container ID"),
    },
  }, ({ list_ids, parent_list_id, container_id }) =>
    setListPlacement(api, { list_ids, parent_list_id, container_id }));

  server.registerTool("enable_list_feature", {
    description: tr("tool-enable-list-feature", locale),
    inputSchema: {
      list_id: z.string().describe("The list ID"),
      feature: z.enum(["quantity", "deadlines"]).describe("Feature to enable"),
      has_start_date: z.boolean().optional().describe("Show start date field (default false)"),
      has_deadline: z.boolean().optional().describe("Show deadline field (default true)"),
      has_hard_deadline: z.boolean().optional().describe("Show hard deadline field (default false)"),
      unit_default: z.string().optional().describe("Default unit label, e.g. 'szt', 'kg'"),
    },
  }, async ({ list_id, feature, has_start_date, has_deadline, has_hard_deadline, unit_default }) => {
    const config = feature === "deadlines"
      ? {
          has_start_date: has_start_date ?? false,
          has_deadline: has_deadline ?? true,
          has_hard_deadline: has_hard_deadline ?? false,
        }
      : unit_default
        ? { unit_default }
        : {};
    return callTool(api, "POST", `/api/lists/${list_id}/features/${feature}`, { config });
  });

  server.registerTool("disable_list_feature", {
    description: tr("tool-disable-list-feature", locale),
    inputSchema: {
      list_id: z.string().describe("The list ID"),
      feature: z.enum(["quantity", "deadlines"]).describe("Feature to disable"),
    },
  }, ({ list_id, feature }) =>
    callTool(api, "DELETE", `/api/lists/${list_id}/features/${feature}`));

  async function setListPlacement(
    api: ApiContext,
    input: { list_ids: string[]; parent_list_id?: string | null; container_id?: string | null }
  ) {
    const list_ids = dedupeIds(input.list_ids);
    if (list_ids.length === 0) {
      return errorResult("Provide at least one list_id.");
    }
    if (input.parent_list_id && input.container_id) {
      return errorResult("Provide either parent_list_id or container_id, not both.");
    }
    return callJsonTool(api, "PATCH", "/api/lists/placement", {
      list_ids,
      parent_list_id: input.parent_list_id ?? null,
      container_id: input.container_id ?? null,
    });
  }
}

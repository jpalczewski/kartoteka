import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { ApiContext } from "../api";
import { callTool, apiCall, errorResult, jsonResult } from "../api";
import { tr } from "../i18n";

const itemTitleSchema = z.string().trim().min(1).max(255);
const quantitySchema = z.number().int().positive();
const actualQuantitySchema = z.number().int().nonnegative();

export function registerItemTools(server: McpServer, api: ApiContext, locale: string): void {
  server.registerTool("get_list_items", {
    description: tr("tool-get-items", locale),
    inputSchema: {
      list_id: z.string().describe("The list ID"),
      completed: z.boolean().optional().describe("Filter by completion state"),
      has_deadline: z.boolean().optional().describe("Filter by presence of deadline"),
      from: z.string().optional().describe("Keep items dated on or after YYYY-MM-DD"),
      to: z.string().optional().describe("Keep items dated on or before YYYY-MM-DD"),
      field: z.enum(["all", "start_date", "deadline", "hard_deadline"]).optional()
        .describe("Date field to use with from/to (default: all)"),
    },
  }, ({ list_id, completed, has_deadline, from, to, field }) => {
    const params = new URLSearchParams();
    if (completed !== undefined) params.set("completed", String(completed));
    if (has_deadline !== undefined) params.set("has_deadline", String(has_deadline));
    if (from) params.set("date_from", from);
    if (to) params.set("date_to", to);
    if (field) params.set("date_field", field);

    const query = params.toString();
    const path = query
      ? `/api/lists/${list_id}/items?${query}`
      : `/api/lists/${list_id}/items`;
    return callTool(api, "GET", path);
  });

  server.registerTool("add_item", {
    description: tr("tool-add-item", locale),
    inputSchema: {
      list_id: z.string().describe("The list ID"),
      title: itemTitleSchema.describe("Item title"),
      description: z.string().optional().describe("Item description"),
      quantity: quantitySchema.optional().describe("Target quantity"),
      unit: z.string().optional().describe("Unit of measurement"),
      start_date: z.string().optional().describe("Start date YYYY-MM-DD"),
      start_time: z.string().optional().describe("Start time HH:MM"),
      deadline: z.string().optional().describe("Deadline YYYY-MM-DD"),
      deadline_time: z.string().optional().describe("Deadline time HH:MM"),
      hard_deadline: z.string().optional().describe("Hard deadline YYYY-MM-DD"),
    },
  }, async ({ list_id, ...fields }) => {
    return withAutoEnable(api, list_id, fields, (f) =>
      apiCall(api, "POST", `/api/lists/${list_id}/items`, f)
    );
  });

  server.registerTool("update_item", {
    description: tr("tool-update-item", locale),
    inputSchema: {
      list_id: z.string().describe("The list ID"),
      item_id: z.string().describe("The item ID"),
      title: itemTitleSchema.optional().describe("New title"),
      description: z.string().nullable().optional().describe("New description (null to clear)"),
      completed: z.boolean().optional().describe("Completion state"),
      quantity: quantitySchema.optional().describe("Target quantity"),
      actual_quantity: actualQuantitySchema.optional().describe("Actual quantity (auto-completes when >= quantity)"),
      unit: z.string().nullable().optional().describe("Unit (null to clear)"),
      start_date: z.string().nullable().optional().describe("Start date YYYY-MM-DD (null to clear)"),
      start_time: z.string().nullable().optional().describe("Start time HH:MM (null to clear)"),
      deadline: z.string().nullable().optional().describe("Deadline YYYY-MM-DD (null to clear)"),
      deadline_time: z.string().nullable().optional().describe("Deadline time HH:MM (null to clear)"),
      hard_deadline: z.string().nullable().optional().describe("Hard deadline YYYY-MM-DD (null to clear)"),
    },
  }, async ({ list_id, item_id, ...fields }) => {
    return withAutoEnable(api, list_id, fields, (f) =>
      apiCall(api, "PUT", `/api/lists/${list_id}/items/${item_id}`, f)
    );
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
    callTool(api, "PATCH", `/api/items/${item_id}/move`, { target_list_id }));

  async function withAutoEnable(
    api: ApiContext,
    listId: string,
    fields: Record<string, unknown>,
    apiFn: (f: Record<string, unknown>) => Promise<Response>
  ): Promise<{ content: { type: "text"; text: string }[]; isError?: boolean }> {
    const res = await apiFn(fields);
    if (!res.ok) {
      const raw = await res.text();
      if (res.status === 422) {
        let body: { error?: string; feature?: string; message?: string } = {};
        try { body = JSON.parse(raw) as typeof body; } catch { /* ignore */ }

        if (body.error === "feature_required" && body.feature) {
          let autoEnable = false;
          try {
            const settings = await apiCall(api, "GET", "/api/settings").then(r => r.json()) as Record<string, unknown>;
            autoEnable = settings["mcp_auto_enable_features"] === true;
          } catch { /* default false */ }

          if (autoEnable) {
            const config = body.feature === "deadlines"
              ? { has_start_date: false, has_deadline: true, has_hard_deadline: false }
              : {};
            const enableRes = await apiCall(api, "POST", `/api/lists/${listId}/features/${body.feature}`, { config });
            if (!enableRes.ok) {
              return errorResult(`Failed to auto-enable feature '${body.feature}': ${await enableRes.text()}`);
            }
            const retry = await apiFn(fields);
            if (!retry.ok) {
              return errorResult(`API error ${retry.status}: ${await retry.text()}`);
            }
            try {
              return jsonResult(await retry.json());
            } catch {
              return errorResult("Failed to parse API response after auto-enabling feature.");
            }
          }

          return errorResult(
            `${body.message ?? "Feature not enabled."} Options: (1) use enable_list_feature tool to enable it, (2) retry without the field.`
          );
        }
      }
      return errorResult(`API error ${res.status}: ${raw}`);
    }
    try {
      return jsonResult(await res.json());
    } catch {
      return errorResult("Failed to parse API response.");
    }
  }
}

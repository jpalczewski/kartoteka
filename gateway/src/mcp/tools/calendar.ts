import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { ApiContext } from "../api";
import { callTool } from "../api";
import { tr } from "../i18n";

export function registerCalendarTools(server: McpServer, api: ApiContext, locale: string): void {
  server.registerTool("get_calendar", {
    description: tr("tool-get-calendar", locale),
    inputSchema: {
      from: z.string().describe("Start date YYYY-MM-DD"),
      to: z.string().describe("End date YYYY-MM-DD"),
      field: z.enum(["start_date", "deadline", "hard_deadline"]).optional().describe("Filter by date field (default: all)"),
      mode: z.enum(["counts", "full"]).default("full").describe("'counts' = day summaries, 'full' = complete items"),
    },
  }, ({ from, to, field, mode }) => {
    const params = new URLSearchParams({ from, to, mode });
    if (field) params.set("field", field);
    return callTool(api, "GET", `/api/items/calendar?${params.toString()}`);
  });

  server.registerTool("get_items_by_date", {
    description: tr("tool-get-today", locale),
    inputSchema: {
      date: z.string().describe("Date YYYY-MM-DD"),
      field: z.enum(["start_date", "deadline", "hard_deadline"]).optional().describe("Filter by date field (default: all)"),
      include_overdue: z.boolean().default(true).describe("Include overdue items"),
    },
  }, ({ date, field, include_overdue }) => {
    const params = new URLSearchParams({ date });
    if (field) params.set("field", field);
    if (include_overdue !== undefined) params.set("include_overdue", String(include_overdue));
    return callTool(api, "GET", `/api/items/by-date?${params.toString()}`);
  });
}

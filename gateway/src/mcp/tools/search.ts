import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";

import type { ApiContext } from "../api";
import { callTool } from "../api";
import { tr } from "../i18n";

export function registerSearchTools(server: McpServer, api: ApiContext, locale: string): void {
  server.registerTool("search_entities", {
    description: tr("tool-search-entities", locale),
    inputSchema: {
      query: z.string().trim().min(1).describe("Plain-text query for names/titles and descriptions"),
      limit: z.number().int().positive().max(100).optional().describe("Maximum number of results"),
    },
  }, ({ query, limit }) => {
    const params = new URLSearchParams();
    params.set("query", query.trim());
    if (limit !== undefined) params.set("limit", String(limit));
    return callTool(api, "GET", `/api/search?${params.toString()}`);
  });
}

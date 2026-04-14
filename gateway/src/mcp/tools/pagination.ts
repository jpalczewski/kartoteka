import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { ApiContext } from "../api";
import { callTool } from "../api";
import { tr } from "../i18n";

export function registerPaginationTools(server: McpServer, api: ApiContext, locale: string): void {
  server.registerTool("next_cursor_page", {
    description: tr("tool-next-cursor-page", locale),
    inputSchema: {
      cursor: z.string().min(1).describe("Opaque cursor returned by a paginated Kartoteka tool"),
    },
  }, ({ cursor }) =>
    callTool(api, "GET", `/api/next-page?cursor=${encodeURIComponent(cursor)}`));
}

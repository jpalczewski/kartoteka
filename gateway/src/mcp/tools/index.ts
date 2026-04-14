import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { ApiContext } from "../api";
import { registerListTools } from "./lists";
import { registerItemTools } from "./items";
import { registerContainerTools } from "./containers";
import { registerTagTools } from "./tags";
import { registerCalendarTools } from "./calendar";
import { registerPaginationTools } from "./pagination";
import { registerSearchTools } from "./search";

export function registerTools(server: McpServer, api: ApiContext, locale: string): void {
  registerListTools(server, api, locale);
  registerItemTools(server, api, locale);
  registerContainerTools(server, api, locale);
  registerTagTools(server, api, locale);
  registerCalendarTools(server, api, locale);
  registerPaginationTools(server, api, locale);
  registerSearchTools(server, api, locale);
}

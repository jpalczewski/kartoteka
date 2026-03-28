import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { ApiContext } from "../api";
import { registerListTools } from "./lists";
import { registerItemTools } from "./items";
import { registerContainerTools } from "./containers";
import { registerTagTools } from "./tags";
import { registerCalendarTools } from "./calendar";

export function registerTools(server: McpServer, api: ApiContext): void {
  registerListTools(server, api);
  registerItemTools(server, api);
  registerContainerTools(server, api);
  registerTagTools(server, api);
  registerCalendarTools(server, api);
}

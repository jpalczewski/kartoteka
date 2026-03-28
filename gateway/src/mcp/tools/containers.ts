import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { ApiContext } from "../api";
import { callTool } from "../api";
import { tr } from "../i18n";

export function registerContainerTools(server: McpServer, api: ApiContext, locale: string): void {
  server.registerTool("list_containers", {
    description: tr("tool-list-containers", locale),
    inputSchema: {},
  }, () => callTool(api, "GET", "/api/containers"));

  server.registerTool("create_container", {
    description: tr("tool-create-container", locale),
    inputSchema: {
      name: z.string().describe("Container name"),
      status: z.enum(["active", "done", "paused"]).nullable().default(null).describe("null=folder, active/done/paused=project"),
      parent_container_id: z.string().optional().describe("Parent container ID (for nesting)"),
    },
  }, ({ name, status, parent_container_id }) =>
    callTool(api, "POST", "/api/containers", { name, status, parent_container_id }));

  server.registerTool("get_container", {
    description: tr("tool-get-container", locale),
    inputSchema: {
      container_id: z.string().describe("The container ID"),
    },
  }, ({ container_id }) => callTool(api, "GET", `/api/containers/${container_id}`));

  server.registerTool("get_container_children", {
    description: tr("tool-get-container-children", locale),
    inputSchema: {
      container_id: z.string().describe("The container ID"),
    },
  }, ({ container_id }) => callTool(api, "GET", `/api/containers/${container_id}/children`));

  server.registerTool("get_home", {
    description: tr("tool-get-home", locale),
    inputSchema: {},
  }, () => callTool(api, "GET", "/api/home"));
}

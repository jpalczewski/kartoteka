import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { ApiContext } from "../api";
import { callTool } from "../api";

export function registerContainerTools(server: McpServer, api: ApiContext): void {
  server.registerTool("list_containers", {
    description: "List all containers (folders and projects) for the current user",
    inputSchema: {},
  }, () => callTool(api, "GET", "/api/containers"));

  server.registerTool("create_container", {
    description: "Create a new container (folder or project). Status null=folder, 'active'/'done'/'paused'=project.",
    inputSchema: {
      name: z.string().describe("Container name"),
      status: z.enum(["active", "done", "paused"]).nullable().default(null).describe("null=folder, active/done/paused=project"),
      parent_container_id: z.string().optional().describe("Parent container ID (for nesting)"),
    },
  }, ({ name, status, parent_container_id }) =>
    callTool(api, "POST", "/api/containers", { name, status, parent_container_id }));

  server.registerTool("get_container", {
    description: "Get a container with progress metrics (completed items/lists counts)",
    inputSchema: {
      container_id: z.string().describe("The container ID"),
    },
  }, ({ container_id }) => callTool(api, "GET", `/api/containers/${container_id}`));

  server.registerTool("get_container_children", {
    description: "Get sub-containers and lists inside a container",
    inputSchema: {
      container_id: z.string().describe("The container ID"),
    },
  }, ({ container_id }) => callTool(api, "GET", `/api/containers/${container_id}/children`));

  server.registerTool("get_home", {
    description: "Get home dashboard: pinned items, recent items, root containers and lists",
    inputSchema: {},
  }, () => callTool(api, "GET", "/api/home"));
}

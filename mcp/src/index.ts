// Kartoteka MCP Server — scaffold
// TODO: implement OAuth provider + MCP tools

export interface Env {
    DB: D1Database;
    OAUTH_KV: KVNamespace;
}

export default {
    async fetch(request: Request, env: Env): Promise<Response> {
        const url = new URL(request.url);

        if (url.pathname === "/health") {
            return new Response("ok");
        }

        // TODO: wire up @cloudflare/workers-oauth-provider
        // TODO: wire up @modelcontextprotocol/sdk with tools:
        //   - list_lists
        //   - create_list
        //   - add_item
        //   - toggle_item
        //   - get_list_items

        return new Response("Kartoteka MCP Server — not yet implemented", {
            status: 501,
        });
    },
};

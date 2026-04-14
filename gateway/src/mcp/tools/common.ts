import type { ApiContext } from "../api";
import { apiCall, errorResult, jsonResult, type ToolResult } from "../api";

export function dedupeIds(ids: string[] | undefined | null): string[] {
  if (!ids) return [];
  const seen = new Set<string>();
  const out: string[] = [];
  for (const id of ids) {
    if (!id || seen.has(id)) continue;
    seen.add(id);
    out.push(id);
  }
  return out;
}

export function validateExclusiveTargets(
  values: Array<string[]>,
  missingMessage: string,
  duplicateMessage: string
): string | null {
  const populated = values.filter((ids) => ids.length > 0);
  if (populated.length === 0) return missingMessage;
  if (populated.length > 1) return duplicateMessage;
  return null;
}

export async function callJsonTool(
  api: ApiContext,
  method: string,
  path: string,
  body?: unknown
): Promise<ToolResult> {
  const res = await apiCall(api, method, path, body);
  if (!res.ok) {
    return errorResult(`API error ${res.status}: ${await res.text()}`);
  }
  return jsonResult(await res.json());
}

#!/usr/bin/env python3
"""Parse raw NDJSON logs from `docker logs` (prod/preview/develop containers via SSH).

Usage:
    ssh niechybnie "docker logs <container> --tail 300 2>&1" | python3 scripts/docker-logs.py
    ssh niechybnie "docker logs <container> --tail 300 2>&1" > /tmp/logs.json && python3 scripts/docker-logs.py /tmp/logs.json
    python3 scripts/docker-logs.py /tmp/logs.json [lines] [filter]

Container names (from `docker ps`):
    prod:    y36bsm3jukhpg7i4cw7yp0hf-*   (ghcr.io/jpalczewski/kartoteka:latest)
    develop: hpxmp0eeq02kj3qoqdjud8k8-*   (ghcr.io/jpalczewski/kartoteka:develop)
    preview: wrpu74yku32jdqz74f54ljbq-*   (ghcr.io/jpalczewski/kartoteka:preview)
"""
import json
import re
import sys

src = open(sys.argv[1]) if len(sys.argv) > 1 else sys.stdin
lines_limit = int(sys.argv[2]) if len(sys.argv) > 2 else 0
query = sys.argv[3].lower() if len(sys.argv) > 3 else ""

entries = []
for line in src:
    line = line.strip()
    if not line:
        continue
    try:
        obj = json.loads(line)
        ts = obj.get("timestamp", "")[:19].replace("T", " ")
        level = obj.get("level", "?").upper()
        target = obj.get("target", "")
        fields = obj.get("fields", {})
        span = obj.get("span", {})
        spans = obj.get("spans", [])

        msg = fields.get("message", fields.get("summary", ""))
        if not msg:
            msg = json.dumps(fields)[:80]

        # surface action from handler spans
        action = next((s.get("action") for s in reversed(spans) if "action" in s), None)
        # surface HTTP context from spans
        http = next((f"{s['method']} {s['uri']}" for s in spans if "uri" in s), None)
        # surface MCP tool name from request field
        mcp_tool = None
        if "request" in fields:
            m = re.search(r'name: "(\w+)"', fields["request"])
            if m:
                mcp_tool = m.group(1)

        out = f"{ts}  {level:<5}  {target:<42}  {msg[:70]}"
        if action:
            out += f"  [action={action}]"
        if mcp_tool:
            out += f"  [tool={mcp_tool}]"
        if http and "tower_http" not in target:
            out += f"  ({http})"
        entries.append(out)
    except (json.JSONDecodeError, KeyError):
        entries.append(line[:120])

if query:
    entries = [e for e in entries if query in e.lower()]
if lines_limit:
    entries = entries[-lines_limit:]

for e in entries:
    print(e)
print(f"\n--- {len(entries)} entries ---", file=sys.stderr)

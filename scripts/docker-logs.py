#!/usr/bin/env python3
"""Parse raw NDJSON logs from `docker logs`.

Usage:
    python3 scripts/docker-logs.py <env>              # SSH directly (needs scripts/.docker-env.local)
    python3 scripts/docker-logs.py <env> 100 warn     # custom tail + filter
    python3 scripts/docker-logs.py /tmp/logs.json [lines] [filter]

scripts/.docker-env.local is gitignored — create it with:
    <env-name>=<container-prefix>
    ssh_host=<host>
"""
import json
import re
import sys
import os

sys.path.insert(0, os.path.dirname(__file__))
from _common import known_envs, open_source, strip_leptos_hash

arg1 = sys.argv[1] if len(sys.argv) > 1 else None
envs = known_envs()

if arg1 in envs:
    tail_arg = int(sys.argv[2]) if len(sys.argv) > 2 and sys.argv[2].isdigit() else 300
    query = sys.argv[3].lower() if len(sys.argv) > 3 else ""
    lines_limit = 0
    src = open_source(arg1, tail_arg)
else:
    lines_limit = int(sys.argv[2]) if len(sys.argv) > 2 and sys.argv[2].isdigit() else 0
    query = sys.argv[3].lower() if len(sys.argv) > 3 else ""
    src = open_source(arg1)

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

        if msg == "close" and "time.busy" in fields:
            msg = f"close  busy={fields['time.busy']} idle={fields['time.idle']}"

        action = next((s.get("action") for s in reversed(spans) if "action" in s), None)
        http = next((f"{s['method']} {strip_leptos_hash(s['uri'])}" for s in spans if "uri" in s), None)
        mcp_tool = None
        if "request" in fields:
            m = re.search(r'name: "(\w+)"', fields["request"])
            if m:
                mcp_tool = m.group(1)
        if "stream_duration" in fields:
            msg += f"  [session {fields['stream_duration']}]"

        out = f"{ts}  {level:<5}  {target:<42}  {msg[:90]}"
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

#!/usr/bin/env python3
"""Analyze query performance and HTTP latencies from raw NDJSON docker logs.

Usage:
    ssh niechybnie "docker logs <container> --tail 500 2>&1" > /tmp/logs.json
    python3 scripts/docker-perf.py /tmp/logs.json
"""
import json
import re
import sys
from collections import Counter

src = open(sys.argv[1]) if len(sys.argv) > 1 else sys.stdin
lines = src.readlines()

queries = []
responses = []
mcp_tools = []
warns = []

for line in lines:
    line = line.strip()
    if not line:
        continue
    try:
        obj = json.loads(line)
        target = obj.get("target", "")
        fields = obj.get("fields", {})
        spans = obj.get("spans", [])
        ts = obj.get("timestamp", "")[:19].replace("T", " ")
        level = obj.get("level", "?").upper()

        if target == "sqlx::query":
            elapsed_ms = fields.get("elapsed_secs", 0) * 1000
            http = next((f"{s['method']} {s['uri']}" for s in spans if "uri" in s), "internal")
            span = obj.get("span", {})
            span_name = span.get("name", "")
            summary = fields.get("summary", "")[:60]
            queries.append((elapsed_ms, summary, http, span_name, ts))

        elif target == "tower_http::trace::on_response":
            latency_str = fields.get("latency", "0 ms")
            latency_ms = float(re.search(r"[\d.]+", latency_str).group())
            status = fields.get("status", 0)
            span = obj.get("span", {})
            uri = span.get("uri", "")
            method = span.get("method", "")
            if uri != "/health":
                responses.append((latency_ms, status, method, uri, ts))

        elif target == "rmcp::service" and "received request" in fields.get("message", ""):
            m = re.search(r'name: "(\w+)"', fields.get("request", ""))
            if m:
                mcp_tools.append(m.group(1))

        elif level == "WARN":
            msg = fields.get("message", "")
            error = fields.get("error", "")
            warns.append(f"{ts}  {target}  {msg}  {error}")

    except (json.JSONDecodeError, KeyError, AttributeError):
        pass

print("=== SLOW QUERIES (>1ms) ===")
slow = sorted([q for q in queries if q[0] > 1.0], reverse=True)
if slow:
    for ms, summary, http, span_name, ts in slow:
        print(f"  {ms:7.2f}ms  [{span_name:<20}]  {summary:<55}  ({http})")
else:
    print("  none")

print(f"\n=== QUERY STATS ===")
if queries:
    times = [q[0] for q in queries]
    print(f"  count={len(times)}  min={min(times):.3f}ms  avg={sum(times)/len(times):.3f}ms  "
          f"p90={sorted(times)[int(len(times)*0.9)]:.3f}ms  max={max(times):.3f}ms")
else:
    print("  no queries found")

print(f"\n=== NON-HEALTH HTTP RESPONSES ===")
if responses:
    for ms, status, method, uri, ts in sorted(responses, key=lambda x: x[0], reverse=True):
        flag = "  ⚠" if status >= 400 else ""
        print(f"  {ts}  {method} {uri:<35} → {status}  {ms:.0f}ms{flag}")
else:
    print("  none")

print(f"\n=== MCP TOOL USAGE ===")
if mcp_tools:
    for tool, count in Counter(mcp_tools).most_common():
        print(f"  {tool}: {count}x")
else:
    print("  none")

print(f"\n=== WARNINGS ===")
if warns:
    for w in warns:
        print(f"  {w}")
else:
    print("  none")

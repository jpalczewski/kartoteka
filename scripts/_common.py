"""Shared utilities for docker log analysis scripts."""
import io
import os
import re
import subprocess
import sys

_RESERVED_KEYS = {"ssh_host"}


def load_env_map(path=None):
    if path is None:
        path = os.path.join(os.path.dirname(__file__), ".docker-env.local")
    result = {}
    if os.path.exists(path):
        with open(path) as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#") and "=" in line:
                    k, v = line.split("=", 1)
                    result[k.strip()] = v.strip()
    return result


def known_envs():
    """Return env names defined in .docker-env.local (keys excluding reserved ones like ssh_host)."""
    return {k for k in load_env_map() if k not in _RESERVED_KEYS}


def fetch_logs(env, tail=300):
    cfg = load_env_map()
    prefix = cfg.get(env)
    host = cfg.get("ssh_host")
    if not prefix or not host:
        sys.exit(f"Unknown env '{env}' or missing ssh_host. Check scripts/.docker-env.local")
    find = f"docker ps --format '{{{{.Names}}}}' | grep {prefix} | head -1"
    name = subprocess.check_output(["ssh", host, find], text=True).strip()
    if not name:
        sys.exit(f"No running container found for env '{env}' (prefix: {prefix})")
    print(f"[{env}] {name}", file=sys.stderr)
    raw = subprocess.check_output(
        ["ssh", host, f"docker logs {name} --tail {tail} 2>&1"], text=True
    )
    return io.StringIO(raw)


def open_source(arg, tail=300):
    """Return a readable stream: SSH fetch for env names, file open for paths, stdin otherwise."""
    if arg in known_envs():
        return fetch_logs(arg, tail)
    if arg:
        return open(arg)
    return sys.stdin


def strip_leptos_hash(uri):
    """'/leptos/get_item123456789' → '/leptos/get_item'"""
    return re.sub(r"(/leptos/[a-zA-Z_]+)\d+", r"\1", uri)


def parse_duration_ms(s):
    """Parse '341µs', '1.2ms', '3s' → float ms. Returns 0.0 on unrecognised input."""
    s = s.strip()
    if s.endswith("µs"):
        return float(s[:-2]) / 1000
    if s.endswith("ms"):
        return float(s[:-2])
    if s.endswith("s"):
        return float(s[:-1]) * 1000
    return 0.0

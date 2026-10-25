# -*- coding: utf-8 -*-
"""鲸鱼娘状态桥 v2
- /state  : 状态、当前工具、token、三项数值
- /action : 投喂 / 摸头 / 睡觉（POST {"act":"feed"}）
- 数值持久化到 stats.json
"""
import json
import os
import threading
import time
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

QWENPAW = "http://127.0.0.1:64445"
PORT = 7799
HERE = os.path.dirname(os.path.abspath(__file__))
STATS_FILE = os.path.join(HERE, "stats.json")

TOOL_POSE = {
    "read_file": "review", "glob_search": "review", "grep_search": "review",
    "memory_search": "review", "recall_history": "review", "file_reader": "review",
    "write_file": "idea", "edit_file": "idea",
    "execute_shell_command": "debug", "execute_shell": "debug",
    "web_search": "thinking", "web_fetch": "thinking", "browser": "thinking",
    "spawn_subagent": "boss", "chat_with_agent": "meeting",
    "submit_to_agent": "meeting", "list_agents": "slack", "check_agent_task": "slack",
}

lock = threading.Lock()
stats = {"battery": 72, "mood": 80, "satiety": 65, "tokens": 0, "tasks": 0}
if os.path.exists(STATS_FILE):
    try:
        stats.update(json.load(open(STATS_FILE, encoding="utf-8")))
    except Exception:
        pass

state = {
    "status": "idle", "tool": None, "pose": "idle",
    "tokens": {"prompt": 0, "output": 0, "cache_hit": 0, "context": 0},
    "last_turn": None, "finished_at": 0,
    "stats": stats,
}


def save_stats():
    try:
        json.dump(stats, open(STATS_FILE, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
    except Exception:
        pass


def get_json(path, timeout=4):
    return json.loads(urllib.request.urlopen(QWENPAW + path, timeout=timeout).read().decode())


def tail_turn(messages):
    """取最后一条 user 之后的消息（当前轮）"""
    idx = 0
    for i in range(len(messages) - 1, -1, -1):
        if messages[i].get("role") == "user":
            idx = i
            break
    return messages[idx:]


def extract(msgs):
    tool = None
    usage = None
    err = False
    for m in msgs:
        t = m.get("type")
        if t == "plugin_call":
            c = m.get("content") or []
            for b in c if isinstance(c, list) else []:
                d = b.get("data") or {}
                if d.get("name"):
                    tool = d["name"]
        if t == "plugin_call_output":
            st = str(m.get("status") or "").lower()
            blob = json.dumps(m.get("content"), ensure_ascii=False).lower()
            if st in ("error", "failed", "failure") or '"error"' in blob or "traceback" in blob:
                err = True
        meta = (m.get("metadata") or {}).get("metadata") or {}
        u = (meta.get("qwenpaw_turn_usage") or {}).get("usage")
        if u:
            usage = u
    return tool, usage, err


def poller():
    last_status = "idle"
    turn_start = None
    last_usage = None
    last_tick = time.time()
    while True:
        try:
            chats = get_json("/api/chats")
            sr = lambda c: str(c.get("status", "")).lower()
            waiting = [c for c in chats if sr(c) in ("waiting", "waiting_approval", "approval", "pending", "needs_approval")]
            running = [c for c in chats if sr(c) in ("running", "working", "busy")]
            if running:
                c = sorted(running, key=lambda x: x.get("updated_at") or "", reverse=True)[0]
                detail = get_json("/api/chats/%s" % c["id"])
                msgs = tail_turn(detail.get("messages", []))
                tool, usage, err = extract(msgs)
                pose = TOOL_POSE.get(tool, "tool")
                with lock:
                    state["status"] = "working"
                    state["tool"] = tool
                    state["pose"] = pose if tool else "thinking"
                    state["error"] = bool(err)
                    if usage:
                        state["tokens"] = {
                            "prompt": usage.get("last_prompt_tokens") or 0,
                            "output": usage.get("completion_tokens") or 0,
                            "cache_hit": round(usage.get("cache_hit_rate") or 0, 2),
                            "context": usage.get("context_size") or 0,
                        }
                        state["last_turn"] = state["tokens"]
                        last_usage = (state["tokens"]["prompt"], state["tokens"]["output"])
                if last_status != "working":
                    last_status = "working"
                    turn_start = time.time()
                with lock:
                    state["permission"] = bool(waiting)
            else:
                with lock:
                    state["permission"] = bool(waiting)
                    if last_status == "working":
                        state["status"] = "idle"
                        state["pose"] = "success"
                        state["finished_at"] = time.time()
                        state["tool"] = None
                        dur = max(0.5, time.time() - (turn_start or time.time()))
                        out_tok = state["tokens"].get("output") or 0
                        state["turn"] = {
                            "seconds": round(dur, 1),
                            "output": out_tok,
                            "tps": round(out_tok / dur, 1) if out_tok else 0,
                            "cache_hit": state["tokens"].get("cache_hit", 0),
                        }
                        state["last_turn"] = state["turn"]
                        stats["tasks"] += 1
                        stats["mood"] = min(100, stats["mood"] + 4)
                        stats["battery"] = max(0, stats["battery"] - 3)
                        save_stats()
                        state["stats"] = stats
                    elif time.time() - state["finished_at"] > 6 and state["pose"] in ("success",):
                        state["pose"] = "idle"
                last_status = "idle"
        except Exception:
            pass

        # 数值随时间和状态衰减（每 60 秒）
        if time.time() - last_tick > 60:
            with lock:
                stats["satiety"] = max(0, stats["satiety"] - 1)
                if state["status"] == "working":
                    stats["battery"] = max(0, stats["battery"] - 1)
                    stats["mood"] = max(0, stats["mood"] - 1)
                else:
                    stats["battery"] = max(0, stats["battery"] - 0.3)
                if stats["satiety"] < 20:
                    stats["mood"] = max(0, stats["mood"] - 2)
                state["stats"] = {k: round(v, 1) for k, v in stats.items()}
                save_stats()
            last_tick = time.time()
        time.sleep(2)


class H(BaseHTTPRequestHandler):
    def _send(self, obj, code=200):
        body = json.dumps(obj, ensure_ascii=False).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path.startswith("/state"):
            with lock:
                out = dict(state)
                out["stats"] = {k: round(v, 1) for k, v in stats.items()}
                if time.time() - state["finished_at"] > 30:
                    out["last_turn"] = None
            self._send(out)
        elif self.path.startswith("/health"):
            self._send({"ok": True})
        else:
            self._send({"error": "not found"}, 404)

    def do_POST(self):
        n = int(self.headers.get("Content-Length", 0) or 0)
        try:
            body = json.loads(self.rfile.read(n).decode() or "{}")
        except Exception:
            body = {}
        act = body.get("act")
        with lock:
            if act == "feed":
                stats["satiety"] = min(100, stats["satiety"] + 18)
                stats["mood"] = min(100, stats["mood"] + 6)
            elif act == "headpat":
                stats["mood"] = min(100, stats["mood"] + 5)
            elif act == "sleep":
                stats["battery"] = min(100, stats["battery"] + 10)
            save_stats()
            out = {k: round(v, 1) for k, v in stats.items()}
        self._send({"ok": True, "stats": out})

    def do_OPTIONS(self):
        self.send_response(204)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET,POST,OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()

    def log_message(self, *a):
        pass


if __name__ == "__main__":
    threading.Thread(target=poller, daemon=True).start()
    print("bridge v2 on http://127.0.0.1:%d/state" % PORT)
    ThreadingHTTPServer(("127.0.0.1", PORT), H).serve_forever()

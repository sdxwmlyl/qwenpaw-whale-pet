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
    "ver": "v2.1-approval",
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


ACT_FILE = os.path.join(HERE, "activity.json")
activity = {}
try:
    if os.path.exists(ACT_FILE):
        activity.update(json.load(open(ACT_FILE, encoding="utf-8")))
except Exception:
    pass


def save_activity():
    try:
        json.dump(activity, open(ACT_FILE, "w", encoding="utf-8"), ensure_ascii=False)
    except Exception:
        pass


RUN_KEYS = ("running", "working", "busy")
WAIT_KEYS = ("waiting", "waiting_approval", "approval", "pending", "needs_approval")
CELEBRATE_SEC = 8.0
detail_cache = {}


def poller():
    last_tick = time.time()
    last_save = time.time()
    celebrate_until = 0.0
    while True:
        try:
            chats = get_json("/api/chats")
            sr = lambda c: str(c.get("status", "")).lower()

            pend = 0
            try:
                ap = get_json("/api/approval/list", timeout=3)
                if isinstance(ap, dict):
                    pend = int(ap.get("count") or len(ap.get("pending_approvals") or []))
            except Exception:
                pend = 0

            now = time.time()
            fresh_act = {}
            celebrate = False
            waiting_any = False
            for c in chats:
                if c.get("archived"):
                    continue
                cid = c["id"]
                st = "running" if sr(c) in RUN_KEYS else ("waiting" if sr(c) in WAIT_KEYS else "idle")
                waiting_any = waiting_any or st == "waiting"
                rec = activity.get(cid) or {}
                prev = rec.get("status")
                started = rec.get("started") or 0
                if st == "running" and prev != "running":
                    started = now
                if prev == "running" and st != "running":
                    celebrate = True
                fresh_act[cid] = {"status": st, "started": started}
            activity.clear()
            activity.update(fresh_act)

            demo_on = now < (state.get("perm_demo_until") or 0)
            with lock:
                state["approval_pending"] = pend or (1 if demo_on else 0)
                state["permission"] = waiting_any or pend > 0 or demo_on

            running = [c for c in chats if not c.get("archived") and activity.get(c["id"], {}).get("status") == "running"]

            if celebrate:
                with lock:
                    state["status"] = "idle"
                    state["pose"] = "success"
                    state["finished_at"] = now
                    state["tool"] = None
                    state["chat"] = None
                    dur = max(0.5, now - (state.get("turn_started") or now))
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
                celebrate_until = now + CELEBRATE_SEC

            elif running and now >= celebrate_until:
                disp = max(running, key=lambda c: activity.get(c["id"], {}).get("started") or 0)
                # 详情接口返回整个会话（大会话可达十几 MB），必须节流 + 按耗时退避
                cache = detail_cache
                interval = cache.get("every", 8.0)
                if cache.get("cid") != disp["id"] or now - cache.get("at", 0) >= interval:
                    t0 = time.time()
                    try:
                        detail = get_json("/api/chats/%s" % disp["id"], timeout=15)
                    except Exception:
                        detail = {}
                    spent = time.time() - t0
                    cache["every"] = 30.0 if spent > 0.8 else (15.0 if spent > 0.4 else 8.0)
                    tool, usage, err = extract(tail_turn(detail.get("messages") or []))
                    cache.update({"cid": disp["id"], "at": now, "tool": tool, "usage": usage, "err": err})
                tool, usage, err = cache.get("tool"), cache.get("usage"), cache.get("err")
                pose = TOOL_POSE.get(tool, "tool")
                with lock:
                    if state["status"] != "working":
                        state["turn_started"] = now
                    state["status"] = "working"
                    state["chat"] = disp["id"][:8]
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
            else:
                with lock:
                    if now >= celebrate_until:
                        state["status"] = "idle"
                        state["chat"] = None
                    if now - (state.get("finished_at") or 0) > 6 and state["pose"] in ("success",):
                        state["pose"] = "idle"

            if now - last_save > 20:
                save_activity()
                last_save = now
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
            elif act in ("test_sound", "beep"):
                state["beep"] = time.time()
            elif act == "demo_permission":
                state["perm_demo_until"] = time.time() + 8
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
    import socket
    probe = socket.socket()
    try:
        probe.bind(("127.0.0.1", PORT))
    except OSError:
        print("bridge already running on %d, exit" % PORT)
        raise SystemExit(0)
    finally:
        probe.close()
    threading.Thread(target=poller, daemon=True).start()
    print("bridge v2 on http://127.0.0.1:%d/state" % PORT)
    ThreadingHTTPServer(("127.0.0.1", PORT), H).serve_forever()

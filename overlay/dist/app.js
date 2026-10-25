/* 鲸鱼娘 · Tauri 外挂 v3
   - 工具姿势 / 三项数值 / token+tok/s 结算（可常驻，点击关闭）
   - 大小四档显式选择
   - 额外表情：低电、饥饿、连续摸头、连续完成、夜间
*/
const SPRITES = {
  idle: "assets/idle.webp",
  review: "assets/work-review.webp",
  idea: "assets/work-idea.webp",
  debug: "assets/work-debug.webp",
  deploy: "assets/work-deploy.webp",
  meeting: "assets/work-meeting.webp",
  ram: "assets/work-ram.webp",
  deadline: "assets/work-deadline.webp",
  boss: "assets/work-boss.webp",
  slack: "assets/work-slack.webp",
  thinking: "assets/thinking.webp",
  tool: "assets/tool.webp",
  success: "assets/success.webp",
  celebrate: "assets/celebrate.webp",
  sleep: "assets/sleep.webp",
  night: "assets/night.webp",
  headpat: "assets/react-head.webp",
  feed: "assets/eat.webp",
  low: "assets/balance-low.webp",
  angry: "assets/angry.webp",
  failure: "assets/failure.webp",
  blush: "assets/blush.webp",
  star: "assets/star.webp",
  curious: "assets/curious.webp",
  greet: "assets/greet.webp",
  wink: "assets/wink.webp",
  teasing: "assets/teasing.webp",
  bold: "assets/bold.webp",
  pick: "assets/pick-up.webp",
  sweep: "assets/sweep.webp",
  tail: "assets/tail-swing.webp",
  reactTail: "assets/react-tail.webp",
  reactBelly: "assets/react-belly.webp",
  afk: "assets/afk.webp",
  cool: "assets/cool-shades.webp",
  abstract: "assets/abstract.webp",
  waiting: "assets/waiting.webp",
  christmas: "assets/festival-christmas.webp",
  halloween: "assets/festival-halloween.webp",
  midautumn: "assets/festival-mid-autumn.webp",
  spring: "assets/festival-spring.webp",
  valentine: "assets/valentine.webp",
  rain: "assets/weather-rain-happy.webp",
  snow: "assets/weather-snow.webp",
  umbrella: "assets/weather-umbrella.webp",
  thunder: "assets/weather-thunder.webp",
  cold: "assets/weather-cold.webp",
};

const LINES = {
  headpat: ["诶嘿……", "别摸啦，会晕的", "！再来一下"],
  feed: ["好吃！", "谢谢投喂~"],
  low: ["电量有点低了……", "让我歇会儿好不好"],
  hungry: ["肚子饿了……", "想吃点东西"],
  blush: ["别、别一直摸啦！"],
  star: ["今天也很努力呢 ✨", "连续完成，好厉害"],
};

const AUDIO = {
  success: new Audio("audio/success.wav"),
  alert: new Audio("audio/alert.wav"),
};
let soundOn = localStorage.getItem("soundOn") !== "0";
let prevPermission = false;

function play(kind) {
  if (!soundOn) return;
  const a = AUDIO[kind];
  if (!a) return;
  try {
    a.currentTime = 0;
    const p = a.play();
    if (p && p.catch) p.catch(() => say("声音被系统拦了（需要先点一下我）", 2400));
  } catch (e) {}
}

const sprite = document.getElementById("sprite");
const bubble = document.getElementById("bubble");
const menu = document.getElementById("menu");
const pet = document.getElementById("pet");
const hud = document.getElementById("hud");

const bars = {
  battery: document.getElementById("bar-battery"),
  mood: document.getElementById("bar-mood"),
  satiety: document.getElementById("bar-satiety"),
};

let busy = false;
let pose = "idle";
let idleSince = Date.now();
let holdTimer = null;
let bubbleTimer = null;
let sticky = false;
let dragging = false, moved = false, downPos = null;
let lastShownKey = "";
let headpatTimes = [];
let streak = 0;

function say(text, ms = 2600, hint = "") {
  bubble.innerHTML = "";
  bubble.append(document.createTextNode(text));
  if (hint) {
    const s = document.createElement("span");
    s.className = "hint";
    s.textContent = hint;
    bubble.appendChild(s);
  }
  bubble.classList.add("show");
  bubble.classList.toggle("sticky", ms === 0);
  sticky = ms === 0;
  clearTimeout(bubbleTimer);
  if (ms > 0) bubbleTimer = setTimeout(() => bubble.classList.remove("show"), ms);
}

const pick = (a) => a[Math.floor(Math.random() * a.length)];

function setPose(name, holdMs) {
  if (!SPRITES[name]) name = "idle";
  pose = name;
  sprite.src = SPRITES[name];
  document.body.classList.toggle("working", ["review", "idea", "debug", "deploy", "meeting", "ram", "deadline", "boss", "slack", "thinking", "tool"].includes(name));
  document.body.classList.toggle("sleeping", name === "sleep");
  if (holdMs) {
    clearTimeout(holdTimer);
    holdTimer = setTimeout(() => setPose(busy ? "review" : "idle"), holdMs);
  }
}

function setStats(s) {
  if (!s) return;
  const put = (el, v, color) => {
    el.style.width = Math.max(2, Math.min(100, v)) + "%";
    el.style.background = v < 20 ? "#ff8080" : color;
  };
  document.getElementById("v-battery").textContent = Math.round(s.battery);
  document.getElementById("v-mood").textContent = Math.round(s.mood);
  document.getElementById("v-satiety").textContent = Math.round(s.satiety);
  put(bars.battery, s.battery, "#7fe3c0");
  put(bars.mood, s.mood, "#ffb3c7");
  put(bars.satiety, s.satiety, "#ffd27a");

  if (!busy) {
    if (s.battery < 18 && pose !== "low" && pose !== "sleep") {
      setPose("low", 3000);
      say(pick(LINES.low), 3000);
    } else if (s.satiety < 15 && pose !== "angry" && pose !== "sleep") {
      setPose("angry", 3000);
      say(pick(LINES.hungry), 3000);
    }
  }
}

async function poll() {
  try {
    const d = await (await fetch("http://127.0.0.1:7799/state", { cache: "no-store" })).json();
    setStats(d.stats);
    if (d.error && !busy && pose !== "angry") {
      setPose("failure", 2600);
      say("出错了…我看看日志", 2600);
    }
    if (d.permission && !prevPermission) play("alert");
    prevPermission = !!d.permission;

    const working = d.status === "working";
    if (working) {
      idleSince = Date.now();
      if (!busy) {
        busy = true;
        if (sticky) { bubble.classList.remove("show", "sticky"); sticky = false; }
      }
      if (!["headpat", "feed", "blush"].includes(pose)) setPose(d.pose || "review");
      return;
    }

    if (busy) {
      busy = false;
      streak += 1;
      setPose("success", 2200);
      const t = d.last_turn;
      const key = t ? String(t.output) + ":" + String(t.tps) : "";
      if (t && key !== lastShownKey) {
        lastShownKey = key;
        const parts = ["搞定！本轮 " + Number(t.output).toLocaleString() + " tokens"];
        if (t.tps) parts.push(t.tps + " tok/s");
        if (t.seconds) parts.push(t.seconds + " 秒");
        if (t.cache_hit) parts.push("缓存 " + t.cache_hit + "%");
        say(parts.join(" · "), 0, "点我关闭");
      } else {
        say(pick(["搞定啦！", "任务完成"]), 2400);
      }
      play(d.error ? "alert" : "success");
      if (streak >= 3) {
        streak = 0;
        setTimeout(() => { setPose("star", 2600); say(pick(LINES.star), 3000); }, 2600);
      }
      return;
    }

    if (["review", "idea", "debug", "deploy", "meeting", "ram", "deadline", "boss", "slack", "thinking", "tool"].includes(pose)) setPose("idle");
    if (pose === "sleep" && Date.now() - idleSince < 10 * 60 * 1000) setPose("idle");

    const hour = new Date().getHours();
    if (Date.now() - idleSince > 10 * 60 * 1000 && pose !== "sleep" && !sticky) {
      setPose(hour >= 23 || hour < 6 ? "night" : "sleep");
    }
  } catch (e) {}
}

/* 交互 */
pet.addEventListener("mousedown", (e) => {
  if (e.button !== 0) return;
  dragging = true; moved = false;
  downPos = { x: e.screenX, y: e.screenY };
  const w = window.__TAURI__ && window.__TAURI__.window;
  if (w) w.getCurrentWindow().startDragging();
});
window.addEventListener("mousemove", (e) => {
  if (!dragging || !downPos) return;
  if (Math.abs(e.screenX - downPos.x) > 4 || Math.abs(e.screenY - downPos.y) > 4) moved = true;
});
window.addEventListener("mouseup", (e) => {
  if (!dragging) return;
  dragging = false;
  if (moved || e.button !== 0) return;
  const now = Date.now();
  headpatTimes = headpatTimes.filter((t) => now - t < 12000);
  headpatTimes.push(now);
  if (headpatTimes.length >= 3) {
    headpatTimes = [];
    setPose("blush", 1800);
    say(pick(LINES.blush), 2600);
  } else {
    setPose("headpat", 1400);
    say(pick(LINES.headpat));
  }
  fetch("http://127.0.0.1:7799/action", {
    method: "POST", headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ act: "headpat" }),
  }).catch(() => {});
});

bubble.addEventListener("click", () => {
  bubble.classList.remove("show", "sticky");
  sticky = false;
});

window.addEventListener("contextmenu", (e) => { e.preventDefault(); menu.hidden = false; });

function applySize(px) {
  localStorage.setItem("petSize", String(px));
  const w = window.__TAURI__ && window.__TAURI__.window;
  if (w) w.getCurrentWindow().setSize(new w.LogicalSize(px, px + 64));
}

menu.addEventListener("click", async (e) => {
  const act = e.target.dataset.act;
  const size = e.target.dataset.size;
  const w = window.__TAURI__ && window.__TAURI__.window;
  if (size) { applySize(Number(size)); menu.hidden = true; return; }
  if (!act) return;
  menu.hidden = true;
  if (act === "feed") {
    setPose("feed", 1600); say(pick(LINES.feed));
    fetch("http://127.0.0.1:7799/action", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ act: "feed" }) }).catch(() => {});
  } else if (act === "headpat") {
    setPose("headpat", 1400); say(pick(LINES.headpat));
    fetch("http://127.0.0.1:7799/action", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ act: "headpat" }) }).catch(() => {});
  } else if (act === "sleep") {
    setPose("sleep");
    fetch("http://127.0.0.1:7799/action", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ act: "sleep" }) }).catch(() => {});
  } else if (act === "sound") {
    soundOn = !soundOn;
    localStorage.setItem("soundOn", soundOn ? "1" : "0");
    say(soundOn ? "声音已开" : "声音已关", 1800);
    if (soundOn) play("success");
  } else if (act === "hud") {
    hud.hidden = !hud.hidden;
    localStorage.setItem("hudHidden", hud.hidden ? "1" : "0");
  } else if (act === "hide") {
    if (w) w.getCurrentWindow().hide();
  } else if (act === "quit") {
    if (w) w.getCurrentWindow().close();
  }
});

document.addEventListener("click", (e) => {
  if (!menu.hidden && !menu.contains(e.target)) menu.hidden = true;
});

(function init() {
  applySize(Number(localStorage.getItem("petSize") || 240));
  if (localStorage.getItem("hudHidden") === "1") hud.hidden = true;
})();

/* 悬停好奇 */
pet.addEventListener("mouseenter", () => {
  if (!busy && !["sleep", "low", "angry"].includes(pose)) setPose("curious", 1800);
});

/* 双击调戏 */
pet.addEventListener("dblclick", () => {
  setPose("teasing", 1600);
  say("干嘛啦~", 1800);
});

/* 空闲小动作 */
function fidget() {
  if (busy || sticky || ["sleep", "low", "angry", "feed", "headpat"].includes(pose)) return;
  const pool = ["tail", "reactTail", "reactBelly", "wink", "sweep", "cool"];
  const memes = ["meme-peace", "meme-smug", "meme-wakuwaku", "meme-kyun", "meme-music", "meme-doge"];
  if (Math.random() < 0.12 && SPRITES[memes[0]]) {
    const m = memes[Math.floor(Math.random() * memes.length)];
    setPose(m, 3000);
    say("……", 1800);
    return;
  }
  setPose(pool[Math.floor(Math.random() * pool.length)], 2400);
}
setInterval(() => { if (Math.random() < 0.45) fidget(); }, 45000);

/* 节日（固定公历；春节/中秋按 2026 年日期） */
function festivalToday() {
  const d = new Date();
  const m = d.getMonth() + 1, day = d.getDate();
  if (m === 12 && day >= 24 && day <= 26) return "christmas";
  if (m === 10 && day === 31) return "halloween";
  if (m === 2 && day === 14) return "valentine";
  if (m === 9 && day >= 24 && day <= 26) return "midautumn";
  if (m === 2 && day >= 16 && day <= 18) return "spring";
  return null;
}

window.addEventListener("keydown", (e) => {
  if (e.key === "F10") { play("success"); say("成功音 · 测试", 1600); }
  if (e.key === "F11") { play("alert"); say("提醒音 · 测试", 1600); }
  if (e.key === "F9") {
    say("搞定！本轮 16,538 tokens · 23.4 tok/s · 86 秒 · 缓存 99.96%", 0, "点我关闭");
  }
});

setInterval(poll, 2000);
poll();
setTimeout(() => {
  const f = festivalToday();
  if (f) { setPose(f, 4000); say("节日快乐~", 3000); }
  else if (!busy) { setPose("greet", 2200); say("鲸鱼娘上线啦~", 2200); }
}, 600);

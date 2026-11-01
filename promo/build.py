# -*- coding: utf-8 -*-
"""v2 promo: TTS narration -> timeline -> frames -> mp4 with voice."""
import json, os, subprocess, sys, time, urllib.request, wave
sys.stdout.reconfigure(encoding="utf-8")
from playwright.sync_api import sync_playwright

ROOT = r"D:\ProjectWhalePet\promo"
TTS = "http://127.0.0.1:9880/tts"
REF = r"D:\ProjectTTS\gpt-sovits\refs\nahida_ref\136080_audio.wav"
PROMPT = "其实我已经做过一些尝试，不过我现在倒想先听听你们的想法。"
FPS = 30

LINES = [
    ("hook",   "她不是壁纸。",                                   ""),
    ("reveal", "现在，QwenPaw 终于有自己的鲸鱼娘了。",           "QwenPaw 终于有自己的鲸鱼娘了"),
    ("v1",     "你在写代码、查资料，她就换一个姿势陪着你。",     "你在干什么，她就换什么姿势"),
    ("v2",     "这一轮花多少、跑多快，她当场报给你。",           "这一轮花了多少，她当场报"),
    ("v3",     "任务跑完，或者需要你点一下，她会叫你。",         "跑完、需要你时，她会叫一声"),
    ("v4",     "写代码、查资料、跑长任务，甚至摸鱼，她都陪着你。", "写代码、查资料、跑长任务、摸鱼"),
    ("end",    "开源，免费，Windows 双击就能用。",               "开源 · 免费 · Windows 双击就能用"),
]
CAP_IDS = ["", "s1c", "s2c", "s3c", "s4c", "s5c", "s6c"]


def synth(text, out, speed=1.0):
    if os.path.exists(out) and os.path.getsize(out) > 2000:
        return out
    body = {"text": text, "text_lang": "zh", "ref_audio_path": REF, "prompt_text": PROMPT,
            "prompt_lang": "zh", "text_split_method": "cut5", "batch_size": 1,
            "media_type": "wav", "streaming_mode": 0, "parallel_infer": True,
            "temperature": 1, "top_k": 15, "top_p": 1, "speed_factor": speed}
    req = urllib.request.Request(TTS, data=json.dumps(body).encode(),
                                 headers={"Content-Type": "application/json"}, method="POST")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with urllib.request.urlopen(req, timeout=300) as r:
        data = r.read()
    if data[:4] != b"RIFF":
        raise RuntimeError("TTS bad response: %r" % data[:80])
    open(out, "wb").write(data)
    return out


def dur(path):
    with wave.open(path) as w:
        return w.getnframes() / float(w.getframerate())


def main():
    tdir = os.path.join(ROOT, "tts")
    os.makedirs(tdir, exist_ok=True)
    wavs = []
    for key, text, _ in LINES:
        p = synth(text, os.path.join(tdir, key + ".wav"))
        wavs.append((key, p, dur(p)))
        print("tts %-7s %.2fs  %s" % (key, wavs[-1][2], text))

    # timeline: hook gets a beat, later scenes follow their narration
    pads = [0.75, 0.55, 0.55, 0.60, 0.60, 0.60, 1.10]
    scenes, t = [], 0.0
    for i, ((key, p, d), pad) in enumerate(zip(wavs, pads)):
        length = round(d + pad, 2)
        scenes.append({"id": "s%d" % i, "t0": round(t, 2), "t1": round(t + length, 2),
                       "capId": CAP_IDS[i], "cap": LINES[i][2], "wav": p, "at": round(t + 0.12, 2)})
        t += length
    total = round(t, 2)
    TL = {"scenes": scenes, "total": total}
    json.dump(TL, open(os.path.join(ROOT, "timeline_v2.json"), "w", encoding="utf-8"),
              ensure_ascii=False, indent=1)
    print("total %.2fs  scenes %d" % (total, len(scenes)))

    # frames
    frames = os.path.join(ROOT, "frames")
    os.makedirs(frames, exist_ok=True)
    for f in os.listdir(frames):
        os.remove(os.path.join(frames, f))
    N = int(round(total * FPS))
    with sync_playwright() as pw:
        br = pw.chromium.launch(channel="chrome", headless=True,
                                args=["--force-device-scale-factor=1", "--hide-scrollbars",
                                      "--allow-file-access-from-files", "--disable-lcd-text"])
        pg = br.new_page(viewport={"width": 1080, "height": 1920}, device_scale_factor=1)
        pg.goto("file:///" + os.path.join(ROOT, "promo.html").replace("\\", "/"))
        pg.wait_for_function("window.__ready === true", timeout=30000)
        pg.wait_for_timeout(500)
        t0 = time.time()
        for i in range(N):
            pg.evaluate("([t,tl]) => window.__render(t, tl)", [i / FPS, TL])
            pg.screenshot(path=os.path.join(frames, "f_%04d.jpg" % i), type="jpeg",
                          quality=92, animations="disabled")
            if i % 150 == 0:
                print("frame %d/%d %.1fs" % (i, N, time.time() - t0), flush=True)
        br.close()

    # audio bed
    inputs, filters, mix = [], [], []
    for i, s in enumerate(scenes):
        inputs += ["-i", s["wav"]]
        ms = int(round(s["at"] * 1000))
        filters.append("[%d:a]aresample=48000,adelay=%d|%d[a%d]" % (i, ms, ms, i))
        mix.append("[a%d]" % i)
    graph = ";".join(filters) + ";" + "".join(mix) + \
            "amix=inputs=%d:normalize=0,loudnorm=I=-16:TP=-1.5:LRA=11,apad=pad_dur=0.6[aout]" % len(scenes)
    wav_out = os.path.join(ROOT, "voice_v2.m4a")
    cmd = ["ffmpeg", "-y", "-hide_banner", "-v", "error"] + inputs + \
          ["-filter_complex", graph, "-map", "[aout]", "-t", "%.2f" % (total + 0.4),
           "-c:a", "aac", "-b:a", "192k", "-ar", "48000", wav_out]
    subprocess.run(cmd, check=True)

    out = os.path.join(ROOT, os.path.join("out", "鲸鱼娘_宣传_竖版.mp4"))
    subprocess.run(["ffmpeg", "-y", "-hide_banner", "-v", "error",
                    "-framerate", str(FPS), "-i", os.path.join(frames, "f_%04d.jpg"),
                    "-i", wav_out, "-c:v", "libx264", "-preset", "slow", "-crf", "18",
                    "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "192k",
                    "-shortest", "-movflags", "+faststart", out], check=True)
    print("mp4:", out, os.path.getsize(out))


if __name__ == "__main__":
    main()

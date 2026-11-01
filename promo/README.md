# 鲸鱼娘 · 宣传片（竖版 1080×1920）

自建流水线：HTML + CSS + JS 时间轴 → Playwright 逐帧截图 → ffmpeg 编码 + 本地 TTS 配音。
**不依赖任何第三方模板 / 素材 / 付费组件。**

## 成品
- `out/鲸鱼娘_宣传_竖版.mp4` — 1080×1920 / 30fps / **30.9s** / 纳西妲配音 + 解说字幕
- `out/预览_九宫格.png` — 12 帧总览
- `out/archive/` — 早期两版（暗色模板风、无配音），仅作对比

## 重新生成
```bat
cd /d D:\ProjectWhalePet\promo
D:\Miniconda\python.exe build.py
```
- **改画面**：编辑 `promo.html`（大字、小字、版式都在里面，每段是一个 `<section class="sc">`）
- **改解说**：编辑 `build.py` 顶部的 `LINES` —— 第 2 列是 TTS 读的句子，第 3 列是屏幕字幕
- **改节奏**：`LINES` 每句长度决定段落时长；需要固定时长就改 `pads`
- **TTS 缓存**：`tts/*.wav`；改了台词删掉对应 wav 再跑
- **中间帧**：`frames/`（可删，重跑自动重建）

## 结构
```
promo/
├─ promo.html            最终版设计（浅色极简，7 段）
├─ build.py              台词 → TTS → 时间轴 → 逐帧 → 混音 → mp4
├─ assets/
│  ├─ screenshots/       仓库官方截图 4 张（docs/screenshots）
│  └─ poses/             角色姿态 10 张（overlay/dist/assets）
├─ tts/                  生成的配音（gitignored，可重建）
├─ frames/               中间帧（gitignored，可删）
└─ out/                  成片与预览
```

## 分段（7 段）
| 时间 | 画面 | 解说 |
|---|---|---|
| 0–4s | 黑场「她不是壁纸。」 | 她不是壁纸。 |
| 4–8s | 揭晓 + 角色入场 | 现在，QwenPaw 终于有自己的鲸鱼娘了。 |
| 8–13s | 截图：姿态 + 数值 | 你在写代码、查资料，她就换一个姿势陪着你。 |
| 13–17s | 截图：token 气泡 | 这一轮花多少、跑多快，她当场报给你。 |
| 17–22s | 截图：需要你时 | 任务跑完，或者需要你点一下，她会叫你。 |
| 22–28s | 四格场景 | 写代码、查资料、跑长任务，甚至摸鱼，她都陪着你。 |
| 28–31s | 仓库 + 安装 | 开源，免费，Windows 双击就能用。 |

## 依赖
- **Playwright + 系统 Chrome**（`channel="chrome"`，不需要下载 Chromium）
- ffmpeg（只用到 libx264 / aac）
- 本地 TTS：GPT-SoVITS `http://127.0.0.1:9880`（音色参考 `D:\ProjectTTS\gpt-sovits\refs\nahida_ref\136080_audio.wav`）
  起不来时删掉 `tts/` 也能先出无声版（字幕自带）

## 素材与版权
截图、姿态均来自本仓库（MIT）。配音为纳西妲音色，**对外商用建议换成自有音色**。

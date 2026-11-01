import { AbsoluteFill, Audio, Composition, Img, Sequence, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import React from "react";

const NAVY = "#070b12";
const INK = "#e9f0fb";
const MUTED = "#8ea1bb";

type Scene = { no: string; title: string; sub: string; img: string; color: string };

const SCENES: Scene[] = [
  { no: "01", title: "权限提醒", sub: "QwenPaw 要你点头时，她立刻出声并冒气泡", img: "shots/perm.png", color: "#ffcf5c" },
  { no: "02", title: "多会话并行", sub: "几个任务同时跑，她直接报「现在有 N 个会话」", img: "shots/multi.png", color: "#5fe0b0" },
  { no: "03", title: "切换智能体", sub: "托盘里选谁，她就只盯那一个 agent", img: "shots/agent.png", color: "#6fb6ff" },
  { no: "04", title: "每轮总结", sub: "token / 耗时 / 缓存命中，一次说清", img: "shots/token.png", color: "#ff9dbb" },
];

const Backdrop: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const drift = Math.sin(frame / (fps * 3)) * 60;
  return (
    <AbsoluteFill style={{ background: `radial-gradient(1200px 900px at 70% 8%, #12203a 0%, ${NAVY} 60%)` }}>
      <AbsoluteFill
        style={{
          backgroundImage:
            "linear-gradient(rgba(120,170,255,0.055) 1px, transparent 1px), linear-gradient(90deg, rgba(120,170,255,0.055) 1px, transparent 1px)",
          backgroundSize: "72px 72px",
          transform: `translateY(${drift}px)`,
          maskImage: "linear-gradient(to bottom, rgba(0,0,0,0.9), transparent 85%)",
        }}
      />
      <div
        style={{
          position: "absolute",
          width: 900,
          height: 900,
          borderRadius: "50%",
          left: -260 + Math.sin(frame / 40) * 40,
          top: 980,
          background: "radial-gradient(circle, rgba(90,150,255,0.16), transparent 65%)",
        }}
      />
    </AbsoluteFill>
  );
};

const Intro: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const s = spring({ frame, fps, config: { damping: 200, mass: 0.6 } });
  const y = interpolate(s, [0, 1], [40, 0]);
  const line = interpolate(frame, [10, 34], [0, 1], { extrapolateRight: "clamp" });
  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center" }}>
      <div style={{ opacity: s, transform: `translateY(${y}px)`, textAlign: "center" }}>
        <div style={{ fontSize: 44, letterSpacing: 10, color: MUTED, marginBottom: 26 }}>QWENPA W · DESKTOP PET</div>
        <div style={{ fontSize: 190, fontWeight: 800, color: INK, letterSpacing: 6, lineHeight: 1.05 }}>鲸鱼娘</div>
        <div style={{ display: "flex", justifyContent: "center", alignItems: "center", gap: 22, marginTop: 18 }}>
          <div style={{ height: 6, width: 220 * line, background: "#ffcf5c", borderRadius: 3 }} />
          <div style={{ fontSize: 86, fontWeight: 700, color: "#ffcf5c" }}>v1.2</div>
          <div style={{ height: 6, width: 220 * line, background: "#ffcf5c", borderRadius: 3 }} />
        </div>
        <div style={{ fontSize: 40, color: MUTED, marginTop: 34, letterSpacing: 2 }}>四大新功能 · 一次看懂</div>
      </div>
    </AbsoluteFill>
  );
};

const FeatureScene: React.FC<{ scene: Scene; index: number }> = ({ scene, index }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const inS = spring({ frame: frame - 6, fps, config: { damping: 200, mass: 0.7 } });
  const cardS = spring({ frame: frame - 14, fps, config: { damping: 200, mass: 0.9 } });
  const shine = interpolate(frame, [30, 78], [-0.4, 1.4], { extrapolateRight: "clamp", extrapolateLeft: "clamp" });
  const numIdx = parseInt(scene.no, 10);

  return (
    <AbsoluteFill style={{ padding: "150px 90px" }}>
      <div style={{ display: "flex", alignItems: "flex-end", gap: 28, opacity: inS, transform: `translateX(${interpolate(inS, [0, 1], [-40, 0])}px)` }}>
        <div style={{ fontSize: 150, fontWeight: 800, color: scene.color, lineHeight: 0.9, opacity: 0.95 }}>{scene.no}</div>
        <div style={{ paddingBottom: 18 }}>
          <div style={{ width: 8, height: 96, background: scene.color, borderRadius: 4, marginBottom: 18, transform: `scaleY(${inS})`, transformOrigin: "top" }} />
          <div style={{ fontSize: 92, fontWeight: 800, color: INK, letterSpacing: 2 }}>{scene.title}</div>
        </div>
      </div>
      <div style={{ fontSize: 40, color: MUTED, marginTop: 26, lineHeight: 1.5, opacity: inS, maxWidth: 900 }}>{scene.sub}</div>

      <div
        style={{
          marginTop: 70,
          transform: `perspective(1600px) rotateY(${interpolate(cardS, [0, 1], [10, 0])}deg) translateY(${interpolate(cardS, [0, 1], [60, 0])}px) scale(${interpolate(cardS, [0, 1], [0.94, 1])})`,
          opacity: cardS,
          borderRadius: 40,
          padding: 5,
          background: `linear-gradient(135deg, ${scene.color}66, rgba(255,255,255,0.06))`,
          boxShadow: `0 40px 120px rgba(0,0,0,0.55), 0 0 0 1px rgba(255,255,255,0.05)`,
          position: "relative",
          overflow: "hidden",
          alignSelf: "center",
          width: 760,
        }}
      >
        <div style={{ borderRadius: 36, overflow: "hidden", background: "#0c1220" }}>
          <Img src={staticFile(scene.img)} style={{ width: "100%", display: "block" }} />
        </div>
        <div
          style={{
            position: "absolute",
            top: 0,
            bottom: 0,
            width: 260,
            left: `${shine * 100}%`,
            background: "linear-gradient(90deg, transparent, rgba(255,255,255,0.16), transparent)",
            transform: "skewX(-18deg)",
          }}
        />
      </div>

      <div style={{ position: "absolute", right: 90, bottom: 90, fontSize: 34, color: "rgba(255,255,255,0.35)", letterSpacing: 4 }}>
        {index + 1} / {SCENES.length}
      </div>
    </AbsoluteFill>
  );
};

const Outro: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const s = spring({ frame, fps, config: { damping: 200 } });
  return (
    <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", opacity: s }}>
      <div style={{ fontSize: 108, fontWeight: 800, color: INK }}>鲸鱼娘 v1.2</div>
      <div style={{ display: "flex", gap: 18, marginTop: 40 }}>
        {["权限提醒", "多会话", "切智能体", "每轮总结"].map((t, i) => (
          <div
            key={t}
            style={{
              padding: "14px 26px",
              borderRadius: 999,
              border: "1px solid rgba(255,255,255,0.18)",
              color: INK,
              fontSize: 32,
              opacity: spring({ frame: frame - 8 - i * 5, fps, config: { damping: 200 } }),
            }}
          >
            {t}
          </div>
        ))}
      </div>
      <div style={{ marginTop: 56, fontSize: 34, color: MUTED, letterSpacing: 2 }}>github.com/sdxwmlyl/qwenpaw-whale-pet</div>
      <div style={{ marginTop: 14, fontSize: 28, color: "rgba(255,255,255,0.32)" }}>单文件 · 无杀软误报 · 21MB</div>
    </AbsoluteFill>
  );
};

export const Promo: React.FC = () => {
  const { fps } = useVideoConfig();
  const S = Math.round(3.6 * fps);
  const INTRO = Math.round(2.0 * fps);
  const OUTRO = Math.round(2.4 * fps);
  return (
    <AbsoluteFill style={{ fontFamily: '"Microsoft YaHei", "PingFang SC", sans-serif' }}>
      <Backdrop />
      <Sequence from={0} durationInFrames={INTRO}>
        <Intro />
      </Sequence>
      {SCENES.map((s, i) => (
        <Sequence key={s.no} from={INTRO + i * S} durationInFrames={S}>
          <FeatureScene scene={s} index={i} />
        </Sequence>
      ))}
      <Sequence from={INTRO + SCENES.length * S} durationInFrames={OUTRO}>
        <Outro />
      </Sequence>
      <Audio src={staticFile("audio/bed.wav")} volume={0.22} />
      {SCENES.map((s, i) => (
        <Sequence key={"sfx" + s.no} from={INTRO + i * S} durationInFrames={20}>
          <Audio src={staticFile("audio/click.wav")} volume={0.5} />
        </Sequence>
      ))}
      <Sequence from={INTRO + SCENES.length * S} durationInFrames={30}>
        <Audio src={staticFile("audio/success.wav")} volume={0.55} />
      </Sequence>
    </AbsoluteFill>
  );
};

export const Root: React.FC = () => (
  <Composition id="Promo" component={Promo} durationInFrames={Math.round(2.0 * 30) + 4 * Math.round(3.6 * 30) + Math.round(2.4 * 30)} fps={30} width={1080} height={1920} />
);

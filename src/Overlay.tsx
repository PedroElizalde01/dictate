import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

type Phase = "enter" | "recording" | "transcribing" | "exit";

export default function Overlay() {
  const N = 30
  const [phase, setPhase] = useState<Phase>("enter");
  const [levels, setLevels] = useState<number[]>(new Array(N).fill(0));
  const levelsRef = useRef<number[]>(new Array(N).fill(0));

  useEffect(() => {
    const unlisten1 = listen<{ phase: Phase }>("dictate-phase", (e) => {
      const p = e.payload.phase;
      if (p === "recording") {
        const zeros = new Array(N).fill(0);
        levelsRef.current = zeros;
        setLevels(zeros);
        setPhase("enter");
        requestAnimationFrame(() => requestAnimationFrame(() => setPhase("recording")));
      } else {
        setPhase(p);
      }
    });
    const unlisten2 = listen<number>("audio-level", (e) => {
      const v = Math.min(1, Math.max(0, e.payload));
      const arr = [...levelsRef.current.slice(1), v];
      levelsRef.current = arr;
      setLevels(arr);
    });
    return () => {
      unlisten1.then((f) => f());
      unlisten2.then((f) => f());
    };
  }, []);

  const showSpinner = phase === "transcribing";
  const showBars = phase === "recording" || phase === "enter";

  return (
    <div className={`pill ${phase}`}>
      <div className="bars" aria-hidden={!showBars}>
        {levels.map((v, i) => (
          <span key={i} className="bar" style={{ height: `${10 + v * 90}%` }} />
        ))}
      </div>
      <div className="spinner-wrap" aria-hidden={!showSpinner}>
        <span className="spinner" />
      </div>
    </div>
  );
}

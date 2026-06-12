import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

type Phase = "enter" | "recording" | "transcribing" | "review" | "exit";

export default function Overlay() {
  const N = 30
  const [phase, setPhase] = useState<Phase>("enter");
  const [levels, setLevels] = useState<number[]>(new Array(N).fill(0));
  const levelsRef = useRef<number[]>(new Array(N).fill(0));
  const [text, setText] = useState("");
  const [confirmKey, setConfirmKey] = useState("Tab");
  const taRef = useRef<HTMLTextAreaElement>(null);

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
    const unlisten3 = listen<{ text: string; confirmKey: string }>("review-text", (e) => {
      setText(e.payload.text);
      setConfirmKey(e.payload.confirmKey || "Tab");
      setPhase("review");
      setTimeout(() => taRef.current?.focus(), 90);
    });
    return () => {
      unlisten1.then((f) => f());
      unlisten2.then((f) => f());
      unlisten3.then((f) => f());
    };
  }, []);

  const onReviewKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === confirmKey) {
      e.preventDefault();
      invoke("confirm_review", { text: e.currentTarget.value });
    } else if (e.key === "Escape") {
      e.preventDefault();
      invoke("cancel_review");
    }
  };

  const showSpinner = phase === "transcribing";
  const showBars = phase === "recording" || phase === "enter";

  return (
    <div className={`pill ${phase}`}>
      {phase === "review" ? (
        <div className="review">
          <textarea
            ref={taRef}
            value={text}
            spellCheck={false}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={onReviewKeyDown}
          />
          <div className="review-hint">
            <span className="okbd">{confirmKey}</span> paste
            <span className="sep">·</span>
            <span className="okbd">Esc</span> discard
          </div>
        </div>
      ) : (
        <>
          <div className="bars" aria-hidden={!showBars}>
            {levels.map((v, i) => (
              <span key={i} className="bar" style={{ height: `${10 + v * 90}%` }} />
            ))}
          </div>
          <div className="spinner-wrap" aria-hidden={!showSpinner}>
            <span className="spinner" />
          </div>
        </>
      )}
    </div>
  );
}

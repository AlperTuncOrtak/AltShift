import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./index.css";

type AppSettings = {
  enabled: boolean;
  defaultLayout: string;
  aggressiveness: number;
  blacklist: string[];
};

// ── Fluent Toggle Switch ──────────────────────────────────────────────
function FluentSwitch({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="fluent-switch">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
      />
      <span className="track" />
      <span className="thumb" />
    </label>
  );
}

// ── Fluent Slider ─────────────────────────────────────────────────────
function FluentSlider({
  value,
  onChange,
  onCommit,
}: {
  value: number;
  onChange: (v: number) => void;
  onCommit: (v: number) => void;
}) {
  return (
    <input
      type="range"
      min={0}
      max={100}
      value={value}
      className="w-full h-1 rounded-full appearance-none cursor-pointer"
      style={{
        background: `linear-gradient(to right, hsl(213 100% 62%) ${value}%, rgba(255,255,255,0.15) ${value}%)`,
        outline: "none",
      }}
      onChange={(e) => onChange(Number(e.target.value))}
      onMouseUp={(e) => onCommit(Number((e.target as HTMLInputElement).value))}
    />
  );
}

// ── Fluent Separator ──────────────────────────────────────────────────
const Sep = () => <div className="h-px bg-white/[0.06] mx-4" />;

// ── Main App ──────────────────────────────────────────────────────────
export default function App() {
  const [settings, setSettings] = useState<AppSettings>({
    enabled: true,
    defaultLayout: "us-qwerty",
    aggressiveness: 65,
    blacklist: [],
  });
  const [newApp, setNewApp] = useState("");
  const [corrections, setCorrections] = useState(0);
  const counterRef = useRef<HTMLSpanElement>(null);

  useEffect(() => {
    // Safe invoke that doesn't crash in standard browser
    const safeInvoke = async <T,>(cmd: string, args?: any): Promise<T | null> => {
      try {
        return await invoke<T>(cmd, args);
      } catch (e) {
        console.error(`Invoke ${cmd} failed:`, e);
        return null;
      }
    };

    safeInvoke<AppSettings>("get_settings").then(s => s && setSettings(s));
    
    const id = setInterval(() => {
      safeInvoke<{ total_corrections: number }>("get_stats").then((s) => {
        if (s) {
          setCorrections((prev) => {
            if (s.total_corrections !== prev) {
              counterRef.current?.classList.remove("pop");
              void counterRef.current?.offsetWidth;
              counterRef.current?.classList.add("pop");
            }
            return s.total_corrections;
          });
        }
      });
    }, 1000);
    return () => clearInterval(id);
  }, []);

  const save = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
    const updated = { ...settings, [key]: value };
    setSettings(updated);
    try {
      invoke("update_settings", { settings: updated }).catch(console.error);
    } catch(e) {}
  };

  const handleClose = () => {
    try {
      const appWindow = getCurrentWindow();
      appWindow.hide();
    } catch(e) {
      console.error(e);
    }
  };

  const addApp = () => {
    const app = newApp.trim().toLowerCase();
    if (app && !settings.blacklist.includes(app)) {
      save("blacklist", [...settings.blacklist, app]);
      setNewApp("");
    }
  };

  return (
    <div className="h-screen flex flex-col overflow-hidden text-white/90">
      {/* ── Title Bar (draggable) ── */}
      <div
        className="drag-region flex items-center justify-between px-4 pt-3 pb-2 shrink-0"
        style={{ WebkitAppRegion: "drag" } as React.CSSProperties}
      >
        <div className="flex items-center gap-2.5">
          {/* Logo mark */}
          <div
            className="w-7 h-7 rounded-lg flex items-center justify-center text-xs font-bold"
            style={{ background: "hsl(213 100% 62%)" }}
          >
            AS
          </div>
          <span className="font-semibold text-[15px] tracking-tight">AltShift</span>
        </div>

        <div className="flex items-center gap-1" style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}>
          {/* Status pill */}
          <span
            className="fluent-badge"
            style={
              settings.enabled
                ? { color: "#5af", borderColor: "rgba(85,170,255,0.25)", background: "rgba(85,170,255,0.1)" }
                : { color: "rgba(255,255,255,0.4)" }
            }
          >
            <span
              className={`w-1.5 h-1.5 rounded-full ${settings.enabled ? "bg-[#5af] animate-pulse" : "bg-white/30"}`}
            />
            {settings.enabled ? "Active" : "Paused"}
          </span>

          {/* Window controls */}
          <button
            className="ml-2 w-7 h-7 rounded-md flex items-center justify-center text-white/50 hover:bg-white/10 hover:text-white/80 transition-colors text-[18px] leading-none"
            onClick={handleClose}
            title="Close to tray"
          >
            ×
          </button>
        </div>
      </div>

      {/* ── Body (scrollable) ── */}
      <div className="flex-1 overflow-y-auto px-3 pb-4 space-y-3">

        {/* Stats row */}
        <div className="grid grid-cols-2 gap-2">
          <div className="fluent-card px-4 py-3">
            <div className="text-[11px] font-medium text-white/40 uppercase tracking-widest mb-1">Corrections</div>
            <span ref={counterRef} className="text-2xl font-semibold tabular-nums">
              {corrections.toLocaleString()}
            </span>
          </div>
          <div className="fluent-card px-4 py-3">
            <div className="text-[11px] font-medium text-white/40 uppercase tracking-widest mb-1">Time Saved</div>
            <span className="text-2xl font-semibold tabular-nums">
              {Math.floor(corrections * 1.2)}s
            </span>
          </div>
        </div>

        {/* Engine section */}
        <div>
          <div className="text-[11px] font-medium text-white/35 uppercase tracking-widest px-1 mb-1.5">Engine</div>
          <div className="fluent-card overflow-hidden">

            {/* Master switch */}
            <div className="flex items-center justify-between px-4 py-3 hover:bg-white/[0.03] transition-colors">
              <div>
                <div className="text-[14px] font-medium">Auto-Correct</div>
                <div className="text-[12px] text-white/45">Intercept & fix layout errors</div>
              </div>
              <FluentSwitch checked={settings.enabled} onChange={(v) => save("enabled", v)} />
            </div>

            <Sep />

            {/* Default layout */}
            <div className="flex items-center justify-between px-4 py-3 hover:bg-white/[0.03] transition-colors">
              <div>
                <div className="text-[14px] font-medium">Default Layout</div>
                <div className="text-[12px] text-white/45">Fallback when unknown</div>
              </div>
              <select
                value={settings.defaultLayout}
                onChange={(e) => save("defaultLayout", e.target.value)}
                className="fluent-control text-[13px] font-mono px-2 py-1 text-white/80 cursor-pointer focus:outline-none"
                style={{ minWidth: 110 }}
              >
                <option value="us-qwerty">US QWERTY</option>
                <option value="ru-ycuken">RU YCUKEN</option>
                <option value="de-qwertz">DE QWERTZ</option>
                <option value="fr-azerty">FR AZERTY</option>
              </select>
            </div>

            <Sep />

            {/* Aggressiveness */}
            <div className="px-4 py-3 hover:bg-white/[0.03] transition-colors space-y-2.5">
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-[14px] font-medium">Aggressiveness</div>
                  <div className="text-[12px] text-white/45">Confidence threshold</div>
                </div>
                <span
                  className="text-[13px] font-mono px-2 py-0.5 rounded-md border border-white/10 text-white/70"
                  style={{ background: "rgba(255,255,255,0.06)" }}
                >
                  {settings.aggressiveness}%
                </span>
              </div>
              <FluentSlider
                value={settings.aggressiveness}
                onChange={(v) => setSettings((s) => ({ ...s, aggressiveness: v }))}
                onCommit={(v) => save("aggressiveness", v)}
              />
            </div>
          </div>
        </div>

        {/* Blacklist section */}
        <div>
          <div className="text-[11px] font-medium text-white/35 uppercase tracking-widest px-1 mb-1.5">
            Excluded Apps
          </div>
          <div className="fluent-card px-4 py-3 space-y-3">
            <p className="text-[12px] text-white/45">
              AltShift stays silent in these apps — passwords, games, IDEs.
            </p>

            {/* Add input */}
            <div className="flex gap-2">
              <input
                type="text"
                placeholder="e.g. devenv.exe"
                value={newApp}
                onChange={(e) => setNewApp(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && addApp()}
                className="flex-1 fluent-control px-3 py-1.5 text-[13px] font-mono text-white/80 focus:outline-none placeholder:text-white/25"
              />
              <button
                onClick={addApp}
                className="fluent-control px-3 py-1.5 text-[13px] font-medium text-white/80 hover:text-white transition-colors"
              >
                Block
              </button>
            </div>

            {/* Tags */}
            <div className="flex flex-wrap gap-1.5">
              {settings.blacklist.map((app) => (
                <div
                  key={app}
                  className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[12px] font-mono"
                  style={{
                    background: "rgba(255,80,80,0.08)",
                    border: "1px solid rgba(255,80,80,0.18)",
                    color: "rgba(255,140,140,0.9)",
                  }}
                >
                  {app}
                  <button
                    onClick={() => save("blacklist", settings.blacklist.filter((a) => a !== app))}
                    className="opacity-50 hover:opacity-100 transition-opacity leading-none"
                  >
                    ×
                  </button>
                </div>
              ))}
              {settings.blacklist.length === 0 && (
                <span className="text-[12px] text-white/25 italic">None</span>
              )}
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="text-center text-[11px] text-white/20 pt-1">
          No network · No telemetry · Open source
        </div>
      </div>
    </div>
  );
}

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Switch } from "@/components/ui/switch";
import { Slider } from "@/components/ui/slider";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Keyboard, ShieldAlert, Zap, X, Activity } from "lucide-react";

type AppSettings = {
  enabled: boolean;
  defaultLayout: string;
  aggressiveness: number;
  blacklist: string[];
};

export default function App() {
  const [settings, setSettings] = useState<AppSettings>({
    enabled: true,
    defaultLayout: "us-qwerty",
    aggressiveness: 65,
    blacklist: [],
  });
  const [newApp, setNewApp] = useState("");

  useEffect(() => {
    invoke<AppSettings>("get_settings").then(setSettings).catch(console.error);
  }, []);

  const updateSetting = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
    const updated = { ...settings, [key]: value };
    setSettings(updated);
    invoke("update_settings", { settings: updated }).catch(console.error);
  };

  const addApp = () => {
    const app = newApp.trim().toLowerCase();
    if (app && !settings.blacklist.includes(app)) {
      updateSetting("blacklist", [...settings.blacklist, app]);
      setNewApp("");
    }
  };

  const removeApp = (appToRemove: string) => {
    updateSetting("blacklist", settings.blacklist.filter(app => app !== appToRemove));
  };

  return (
    <div className="min-h-screen bg-zinc-950 text-zinc-100 flex justify-center p-6 sm:p-12 font-sans selection:bg-indigo-500/30">
      <div className="max-w-2xl w-full space-y-8">
        
        {/* Header */}
        <header className="flex items-center gap-4 pb-6 border-b border-white/10">
          <div className="bg-indigo-500 p-2 rounded-xl shadow-lg shadow-indigo-500/20">
            <Keyboard className="w-6 h-6 text-white" />
          </div>
          <div>
            <h1 className="text-2xl font-semibold tracking-tight text-white">AltShift</h1>
            <p className="text-sm text-zinc-400">Offline Privacy Keyboard Switcher</p>
          </div>
          <div className="ml-auto flex items-center gap-2 px-3 py-1.5 rounded-full bg-white/5 border border-white/10">
            <div className={`w-2 h-2 rounded-full ${settings.enabled ? 'bg-emerald-400 animate-pulse' : 'bg-zinc-600'}`} />
            <span className="text-xs font-medium text-zinc-300">{settings.enabled ? 'Active' : 'Paused'}</span>
          </div>
        </header>

        {/* Engine Settings */}
        <section className="space-y-4">
          <h2 className="text-sm font-medium text-indigo-400 uppercase tracking-widest flex items-center gap-2">
            <Zap className="w-4 h-4" /> Engine Configuration
          </h2>
          
          <div className="bg-zinc-900/50 border border-white/10 rounded-2xl p-1 overflow-hidden">
            <div className="flex items-center justify-between p-4 hover:bg-white/[0.02] transition-colors rounded-xl">
              <div>
                <h3 className="font-medium text-white">Master Switch</h3>
                <p className="text-sm text-zinc-400">Enable or disable all layout corrections</p>
              </div>
              <Switch 
                checked={settings.enabled} 
                onCheckedChange={(v) => updateSetting("enabled", v)} 
              />
            </div>
            
            <div className="h-px bg-white/5 mx-4" />
            
            <div className="flex items-center justify-between p-4 hover:bg-white/[0.02] transition-colors rounded-xl">
              <div>
                <h3 className="font-medium text-white">Default Layout</h3>
                <p className="text-sm text-zinc-400">Fallback when no app context is known</p>
              </div>
              <Select value={settings.defaultLayout} onValueChange={(v) => updateSetting("defaultLayout", v)}>
                <SelectTrigger className="w-[160px] bg-zinc-950 border-white/10">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="bg-zinc-900 border-white/10">
                  <SelectItem value="us-qwerty">US QWERTY</SelectItem>
                  <SelectItem value="uk-qwerty">UK QWERTY</SelectItem>
                  <SelectItem value="de-qwertz">DE QWERTZ</SelectItem>
                  <SelectItem value="fr-azerty">FR AZERTY</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="h-px bg-white/5 mx-4" />

            <div className="p-4 hover:bg-white/[0.02] transition-colors rounded-xl space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="font-medium text-white">Aggressiveness</h3>
                  <p className="text-sm text-zinc-400">Confidence threshold for auto-correction</p>
                </div>
                <span className="text-sm font-mono bg-zinc-950 px-2 py-1 rounded-md border border-white/10 text-indigo-300">
                  {settings.aggressiveness}%
                </span>
              </div>
              <Slider 
                value={[settings.aggressiveness]} 
                onValueChange={(v) => setSettings({ ...settings, aggressiveness: v[0] })}
                onValueCommit={(v) => updateSetting("aggressiveness", v[0])}
                max={100} 
                className="py-2"
              />
            </div>
          </div>
        </section>

        {/* Blacklist Settings */}
        <section className="space-y-4 pt-4">
          <h2 className="text-sm font-medium text-rose-400 uppercase tracking-widest flex items-center gap-2">
            <ShieldAlert className="w-4 h-4" /> Application Blacklist
          </h2>
          
          <div className="bg-zinc-900/50 border border-white/10 rounded-2xl p-5 space-y-4">
            <p className="text-sm text-zinc-400">
              AltShift will completely ignore keystrokes in these applications to protect passwords, game controls, and IDE shortcuts.
            </p>
            
            <div className="flex gap-2">
              <Input 
                placeholder="e.g. devenv.exe" 
                value={newApp}
                onChange={(e) => setNewApp(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && addApp()}
                className="bg-zinc-950 border-white/10 font-mono text-sm"
              />
              <Button onClick={addApp} variant="secondary" className="bg-white/10 hover:bg-white/20 text-white">
                Block App
              </Button>
            </div>

            <div className="flex flex-wrap gap-2 pt-2">
              {settings.blacklist.map(app => (
                <div key={app} className="flex items-center gap-2 bg-rose-500/10 border border-rose-500/20 text-rose-300 px-3 py-1.5 rounded-lg text-sm font-mono group">
                  {app}
                  <button onClick={() => removeApp(app)} className="text-rose-400/50 hover:text-rose-300 focus:outline-none">
                    <X className="w-4 h-4" />
                  </button>
                </div>
              ))}
              {settings.blacklist.length === 0 && (
                <p className="text-sm text-zinc-500 italic">No applications blocked.</p>
              )}
            </div>
          </div>
        </section>

      </div>
    </div>
  );
}

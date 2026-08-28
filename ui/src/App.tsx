import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { 
  Search, Menu, ArrowLeft, Home, PlaySquare, Volume2, 
  LayoutTemplate, Activity, Music, Lock, Settings, Info, ChevronRight, CheckCircle2 
} from "lucide-react";

type AppSettings = {
  enabled: boolean;
  defaultLayout: string;
  aggressiveness: number;
  blacklist: string[];
};

async function safeInvoke<T>(cmd: string, args?: any): Promise<T | null> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    return null;
  }
}

function SidebarItem({ icon: Icon, label, active, onClick }: any) {
  return (
    <div 
      onClick={onClick}
      className={\lex items-center gap-3 px-3 py-2.5 rounded-md cursor-pointer transition-all relative \}
    >
      <Icon size={18} strokeWidth={active ? 2.5 : 2} className={active ? "text-[#5af]" : ""} />
      <span className="text-[14px]">{label}</span>
      {active && <div className="absolute left-0 w-1 h-4 bg-[#5af] rounded-r-full" />}
    </div>
  );
}

function Card({ icon: Icon, title, subtitle, badge, badgeColor }: any) {
  return (
    <div className="fluent-card p-4 flex items-center justify-between cursor-pointer group hover:bg-white/[0.08] transition-colors">
      <div className="flex items-center gap-4">
        <Icon size={24} className="text-white/80 group-hover:text-white transition-colors" strokeWidth={1.5} />
        <div>
          <div className="flex items-center gap-2">
            <span className="font-semibold text-[14px] text-white/90">{title}</span>
            {badge && (
              <span 
                className="text-[10px] font-bold px-1.5 py-0.5 rounded-full leading-none"
                style={{ backgroundColor: badgeColor || "hsl(213 100% 62%)", color: "#fff" }}
              >
                {badge}
              </span>
            )}
          </div>
          <div className="text-[12px] text-white/50">{subtitle}</div>
        </div>
      </div>
      <ChevronRight size={18} className="text-white/30 group-hover:text-white/60 transition-colors" />
    </div>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = useState("Ana Sayfa");
  
  useEffect(() => {
    try {
      const appWindow = getCurrentWindow();
      appWindow.show();
    } catch(e) {
      console.log("Tarayıcı modunda çalışıyor.");
    }
  }, []);

  const handleClose = () => {
    try {
      getCurrentWindow().hide();
    } catch(e) {}
  };

  return (
    <div className="h-screen flex text-white/90 font-sans" style={{ background: "transparent" }}>
      
      {/* SIDEBAR */}
      <div className="w-[280px] h-full flex flex-col pt-3 pb-4 border-r border-white/5 bg-[#141414]/90 backdrop-blur-xl relative">
        {/* Back & Menu */}
        <div className="drag-region px-5 pt-2 mb-6 flex flex-col gap-6" style={{ WebkitAppRegion: "drag" } as any}>
          <ArrowLeft size={20} className="text-white/50 hover:text-white cursor-pointer" style={{ WebkitAppRegion: "no-drag" } as any} />
          <Menu size={22} className="text-white/70 hover:text-white cursor-pointer" style={{ WebkitAppRegion: "no-drag" } as any} />
        </div>

        {/* Navigation */}
        <div className="flex-1 overflow-y-auto px-3 space-y-1">
          <SidebarItem icon={Home} label="Ana Sayfa" active={activeTab === "Ana Sayfa"} onClick={() => setActiveTab("Ana Sayfa")} />
          <SidebarItem icon={PlaySquare} label="Motor & Arayüz" active={activeTab === "Motor"} onClick={() => setActiveTab("Motor")} />
          <SidebarItem icon={Volume2} label="Ses Açılır Penceresi" />
          <SidebarItem icon={LayoutTemplate} label="Görev Çubuğu Widget" />
          <SidebarItem icon={Activity} label="Görev Çubuğu Görselleştiricisi" />
          <SidebarItem icon={Music} label="Sıradaki Açılır Penceresi" />
          <SidebarItem icon={Lock} label="Kilitleme Tuşları" />
          <SidebarItem icon={Settings} label="Sistem" />
        </div>

        {/* About */}
        <div className="px-3 mt-auto">
          <SidebarItem icon={Info} label="Hakkında" />
        </div>
      </div>

      {/* MAIN CONTENT */}
      <div className="flex-1 flex flex-col h-full bg-[#1c1c1f]/95 relative overflow-hidden">
        
        {/* TITLE BAR */}
        <div className="drag-region h-[48px] flex items-center justify-between px-4 shrink-0" style={{ WebkitAppRegion: "drag" } as any}>
          <div className="flex items-center gap-3 w-[240px]">
            <div className="w-6 h-6 rounded flex items-center justify-center text-[9px] font-bold" style={{ background: "hsl(213 100% 62%)" }}>AS</div>
            <span className="font-semibold text-[13px]">AltShift</span>
          </div>
          
          {/* Fake Search Bar */}
          <div className="w-[320px] h-8 bg-white/5 hover:bg-white/10 border border-white/5 rounded-md flex items-center px-3 gap-2 transition-colors cursor-text" style={{ WebkitAppRegion: "no-drag" } as any}>
            <Search size={14} className="text-white/40" />
            <input 
              type="text" 
              placeholder="Ayarlarda ara" 
              className="bg-transparent border-none outline-none text-[13px] w-full text-white placeholder-white/40"
            />
          </div>
          
          <div className="w-[240px] flex justify-end">
            <div className="flex items-center" style={{ WebkitAppRegion: "no-drag" } as any}>
              <button className="w-[46px] h-8 flex items-center justify-center text-white/50 hover:bg-white/10 hover:text-white transition-colors text-[18px] leading-none mb-1">−</button>
              <button className="w-[46px] h-8 flex items-center justify-center text-white/50 hover:bg-white/10 hover:text-white transition-colors text-[14px]">□</button>
              <button className="w-[46px] h-8 flex items-center justify-center text-white/50 hover:bg-red-500 hover:text-white transition-colors text-[16px]" onClick={handleClose}>✕</button>
            </div>
          </div>
        </div>

        {/* SCROLLABLE BODY */}
        <div className="flex-1 overflow-y-auto px-[50px] py-8">
          <div className="flex items-baseline gap-3 mb-10">
            <h1 className="text-[28px] font-semibold tracking-tight text-white">AltShift Ayarları</h1>
            <span className="text-[14px] text-white/40 font-medium">v1.2.0</span>
          </div>

          <h2 className="text-[24px] font-semibold mb-6 tracking-tight text-white">{activeTab}</h2>

          {/* Banner Mockup */}
          <div className="w-[380px] h-[140px] rounded-xl mb-10 overflow-hidden relative shadow-lg" style={{ background: "linear-gradient(135deg, #1f40aa, #6034e3)" }}>
            <div className="absolute inset-0 flex items-center justify-center font-bold text-2xl text-white/40 shadow-inner">
              AltShift Visual
            </div>
          </div>

          <div className="flex items-center gap-14 mb-10 border-b border-white/[0.08] pb-8">
            <div className="cursor-pointer group">
              <div className="font-semibold text-[15px] mb-1 group-hover:text-white text-white/90 transition-colors">Güncellemeleri Görüntüle</div>
              <div className="text-[13px] text-white/50">Yenilikleri öğrenin</div>
            </div>
            <div className="flex items-center gap-4">
              <CheckCircle2 size={24} className="text-[#4cc26e]" strokeWidth={2.5} />
              <div>
                <div className="font-semibold text-[15px] mb-0.5 text-white/90">Güncel</div>
                <div className="text-[12px] text-white/40">Son denetleme: 27.08.2026</div>
              </div>
            </div>
          </div>

          <h3 className="font-semibold text-[14px] mb-4 text-white/90">Kontrol paneli</h3>
          
          <div className="grid grid-cols-2 gap-3 max-w-[850px]">
            <Card icon={PlaySquare} title="Motor & Doğruluk" subtitle="Aktif" />
            <Card icon={LayoutTemplate} title="Görev Çubuğu Widget" subtitle="Aktif" badge="PREMIUM" />
            <Card icon={Volume2} title="Ses Açılır Penceresi" subtitle="Aktif" />
            <Card icon={Activity} title="İstatistik Görselleştiricisi" subtitle="Devre Dışı" badge="BETA" badgeColor="#8E2DE2" />
            <Card icon={Music} title="Kısayollar & İstisnalar" subtitle="Devre Dışı" />
            <Card icon={Lock} title="Kilitleme Tuşları Açılır Penceresi" subtitle="Devre Dışı" />
            <Card icon={Settings} title="Sistem" subtitle="Yapılandır" />
          </div>

        </div>
      </div>
    </div>
  );
}

import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { 
  Search, Menu, ArrowLeft, Home, PlaySquare, Volume2, 
  LayoutTemplate, Activity, Music, Lock, Settings, Info, ChevronRight, CheckCircle2,
  Heart
} from "lucide-react";

function SidebarIcon({ icon: Icon, active }: any) {
  return (
    <div className={\w-12 h-10 flex items-center justify-center cursor-pointer relative \}>
      {active && (
        <div className="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-4 bg-[#60cdff] rounded-r-full" />
      )}
      <div className={\lex items-center justify-center w-8 h-8 rounded-md transition-colors \}>
        <Icon size={18} strokeWidth={active ? 2 : 1.5} />
      </div>
    </div>
  );
}

function Card({ icon: Icon, title, subtitle, badge, disabled }: any) {
  return (
    <div className="h-[72px] bg-white/[0.03] hover:bg-white/[0.05] border border-white/[0.05] rounded-lg p-4 flex items-center justify-between cursor-pointer transition-colors">
      <div className="flex items-center gap-4">
        <div className={\\}>
          <Icon size={22} strokeWidth={1.5} />
        </div>
        <div className="flex flex-col justify-center">
          <div className="flex items-center gap-2">
            <span className="font-medium text-[14px] text-white/90">{title}</span>
            {badge && (
              <span className="text-[10px] font-bold px-1.5 py-0.5 rounded-full leading-none bg-[#60cdff] text-[#1c1c1c]">
                {badge}
              </span>
            )}
          </div>
          <div className="text-[12px] text-white/50">{subtitle}</div>
        </div>
      </div>
      <ChevronRight size={16} className="text-white/30" />
    </div>
  );
}

export default function App() {
  useEffect(() => {
    try {
      getCurrentWindow().show();
    } catch(e) {}
  }, []);

  const handleClose = () => { try { getCurrentWindow().hide(); } catch(e) {} };
  const handleMinimize = () => { try { getCurrentWindow().minimize(); } catch(e) {} };

  return (
    <div className="h-screen flex flex-col text-white font-sans overflow-hidden bg-[#202020]">
      
      {/* TITLE BAR */}
      <div className="h-10 flex items-center justify-between px-3 shrink-0 drag-region" style={{ WebkitAppRegion: "drag" } as any}>
        <div className="flex items-center gap-3 w-[240px]">
          <div className="w-4 h-4 rounded-sm flex items-center justify-center text-[8px] font-bold bg-[#e34c67]">AS</div>
          <span className="text-[12px] text-white/80">AltShift</span>
        </div>
        
        {/* Search Bar */}
        <div className="w-[400px] h-8 bg-white/[0.06] hover:bg-white/[0.08] border border-white/[0.05] rounded-md flex items-center px-3 gap-2 cursor-text transition-colors" style={{ WebkitAppRegion: "no-drag" } as any}>
          <input 
            type="text" 
            placeholder="Ayarlarda ara" 
            className="bg-transparent border-none outline-none text-[13px] w-full text-white placeholder-white/50"
          />
          <Search size={14} className="text-white/50" />
        </div>
        
        {/* Window Controls */}
        <div className="flex w-[240px] justify-end h-full" style={{ WebkitAppRegion: "no-drag" } as any}>
          <button onClick={handleMinimize} className="w-12 h-full flex items-center justify-center text-white/60 hover:bg-white/10">−</button>
          <button className="w-12 h-full flex items-center justify-center text-white/60 hover:bg-white/10 text-[10px]">◻</button>
          <button onClick={handleClose} className="w-12 h-full flex items-center justify-center text-white/60 hover:bg-[#c42b1c] hover:text-white">✕</button>
        </div>
      </div>

      <div className="flex-1 flex overflow-hidden">
        {/* THIN SIDEBAR */}
        <div className="w-[48px] h-full flex flex-col items-center py-2 shrink-0">
          <div className="flex flex-col gap-1 w-full flex-1">
            <SidebarIcon icon={Home} active={true} />
            <SidebarIcon icon={PlaySquare} />
            <SidebarIcon icon={Volume2} />
            <SidebarIcon icon={LayoutTemplate} />
            <SidebarIcon icon={Activity} />
            <SidebarIcon icon={Music} />
            <SidebarIcon icon={Lock} />
            <SidebarIcon icon={Settings} />
          </div>
          <div className="mb-2 w-full">
            <SidebarIcon icon={Info} />
          </div>
        </div>

        {/* MAIN CONTENT */}
        <div className="flex-1 flex flex-col px-8 py-2 overflow-y-auto">
          
          {/* Header */}
          <div className="flex flex-col mb-6">
            <div className="flex flex-col gap-4 mb-4 text-white/60">
              <ArrowLeft size={20} className="cursor-pointer hover:text-white" />
              <Menu size={20} className="cursor-pointer hover:text-white" />
            </div>
            
            <div className="flex items-baseline gap-3">
              <h1 className="text-[28px] font-semibold tracking-tight">AltShift Ayarları</h1>
              <span className="text-[14px] text-white/40">v0.1.1</span>
            </div>
          </div>

          <h2 className="text-[22px] font-semibold mb-6">Ana Sayfa</h2>

          {/* Top Info Banner Section */}
          <div className="flex items-stretch gap-4 mb-8">
            
            {/* Updates Banner */}
            <div className="flex-1 bg-white/[0.03] border border-white/[0.05] rounded-lg p-3 flex items-center gap-4 cursor-pointer hover:bg-white/[0.05] transition-colors">
              <div className="w-[140px] h-[70px] rounded-md overflow-hidden bg-gradient-to-br from-blue-600 to-purple-600 flex items-center justify-center">
                <div className="text-[10px] font-bold text-white/50">Görsel</div>
              </div>
              <div className="flex flex-col justify-center">
                <div className="font-medium text-[15px]">Güncellemeleri Görüntüle</div>
                <div className="text-[13px] text-white/50">Yenilikleri öğrenin</div>
              </div>
            </div>

            {/* Status */}
            <div className="w-[240px] bg-white/[0.03] border border-white/[0.05] rounded-lg p-4 flex items-center gap-3">
              <CheckCircle2 size={24} className="text-[#4cc26e]" />
              <div className="flex flex-col justify-center">
                <div className="font-medium text-[14px]">Güncel</div>
                <div className="text-[12px] text-white/40">Son denetleme: 28.08.2026</div>
              </div>
            </div>

            {/* Supporter */}
            <div className="w-[160px] flex items-center justify-center gap-3 cursor-pointer hover:bg-white/[0.03] rounded-lg transition-colors">
              <Heart size={20} className="text-[#60cdff]" />
              <div className="flex flex-col justify-center">
                <div className="font-medium text-[14px]">Supporter</div>
                <div className="text-[12px] text-white/50">Thank you!</div>
              </div>
            </div>

          </div>

          <h3 className="font-medium text-[15px] mb-3 text-white/90">Kontrol paneli</h3>
          
          <div className="grid grid-cols-2 gap-3 pb-8">
            <Card icon={PlaySquare} title="Motor & Kurallar" subtitle="Aktif" />
            <Card icon={LayoutTemplate} title="Görev Çubuğu Widget" subtitle="Aktif" badge="PREMIUM" />
            <Card icon={Volume2} title="Ses Açılır Penceresi" subtitle="Aktif" />
            <Card icon={Activity} title="Görev Çubuğu Görselleştiricisi" subtitle="Devre Dışı" badge="PREMIUM" disabled />
            <Card icon={Music} title="Sıradaki Açılır Penceresi" subtitle="Devre Dışı" disabled />
            <Card icon={Lock} title="Kilitleme Tuşları Açılır Penceresi" subtitle="Devre Dışı" disabled />
            <Card icon={Settings} title="Sistem" subtitle="Yapılandır" />
          </div>

        </div>
      </div>
    </div>
  );
}

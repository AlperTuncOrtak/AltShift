import { useState } from "react";

import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { cn } from "@/lib/utils";
import {
  Keyboard,
  Settings as SettingsIcon,
  ShieldOff,
  BarChart3,
  Plus,
  Trash2,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useCallback } from "react";

type AppSettings = {
  enabled: boolean;
  defaultLayout: string;
  aggressiveness: number;
  blacklist: string[];
};

type Tab = "general" | "blacklist" | "statistics";

const tabs = [
  { id: "general" as Tab, label: "General", icon: Settings },
  { id: "blacklist" as Tab, label: "Blacklist", icon: ShieldOff },
  { id: "statistics" as Tab, label: "Statistics", icon: BarChart3 },
];

export default function App() {
  const [activeTab, setActiveTab] = useState<Tab>("general");

  return (
    <div className="flex h-screen w-full overflow-hidden bg-background text-foreground antialiased dark">
      <aside className="flex w-56 flex-shrink-0 flex-col border-r border-border bg-sidebar">
        <div className="flex h-14 items-center gap-2.5 px-4">
          <div className="flex h-7 w-7 items-center justify-center rounded-md bg-primary text-primary-foreground">
            <Keyboard className="h-4 w-4" />
          </div>
          <span className="text-sm font-semibold tracking-tight">
            AltShift
          </span>
        </div>

        <nav className="flex-1 px-3 py-2">
          <ul className="space-y-0.5">
            {tabs.map((tab) => {
              const Icon = tab.icon;
              const isActive = activeTab === tab.id;
              return (
                <li key={tab.id}>
                  <button
                    type="button"
                    onClick={() => setActiveTab(tab.id)}
                    className={cn(
                      "flex w-full items-center gap-2.5 rounded-md px-2.5 py-1.5 text-sm font-medium transition-colors",
                      isActive
                        ? "bg-secondary text-secondary-foreground"
                        : "text-muted-foreground hover:bg-secondary hover:text-secondary-foreground",
                    )}
                  >
                    <Icon className="h-4 w-4" />
                    {tab.label}
                  </button>
                </li>
              );
            })}
          </ul>
        </nav>

        <div className="border-t border-sidebar-border p-3">
          <p className="text-[10px] leading-tight text-muted-foreground">
            AltShift v2.4.0
          </p>
        </div>
      </aside>

      <main className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-14 items-center border-b border-border px-6">
          <h1 className="text-sm font-semibold">
            {tabs.find((t) => t.id === activeTab)?.label}
          </h1>
        </header>

        <div className="flex-1 overflow-auto p-6">
          <div className="mx-auto max-w-2xl">
            {activeTab === "general" && <GeneralSettings />}
            {activeTab === "blacklist" && <BlacklistSettings />}
            {activeTab === "statistics" && <StatisticsSettings />}
          </div>
        </div>
      </main>
    </div>
  );
}

function GeneralSettings() {
  const [enabled, setEnabled] = useState(true);
  const [defaultLayout, setDefaultLayout] = useState("us-qwerty");
  const [aggressiveness, setAggressiveness] = useState([65]);

  useEffect(() => {
    invoke<AppSettings>("get_settings").then((s) => {
      setEnabled(s.enabled);
      setDefaultLayout(s.defaultLayout);
      setAggressiveness([s.aggressiveness]);
    }).catch(console.error);
  }, []);

  const handleUpdate = (updates: Partial<AppSettings>) => {
    invoke<AppSettings>("get_settings").then((current) => {
      const updated = { ...current, ...updates };
      return invoke("update_settings", { settings: updated });
    }).catch(console.error);
  };

  return (
    <Card className="border border-border bg-card">
      <CardHeader className="pb-4">
        <CardTitle className="text-base font-medium">General Settings</CardTitle>
        <CardDescription className="text-xs">
          Configure how AltShift runs on your system.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-5">
        <div className="flex items-center justify-between gap-4">
          <div className="space-y-0.5">
            <Label htmlFor="enable" className="text-sm font-medium">
              Enable AltShift
            </Label>
            <p className="text-xs text-muted-foreground">
              Start layout correction automatically with Windows.
            </p>
          </div>
          <Switch
            id="enable"
            checked={enabled}
            onCheckedChange={(v) => { setEnabled(v); handleUpdate({ enabled: v }); }}
          />
        </div>

        <div className="h-px bg-border" />

        <div className="space-y-2.5">
          <div className="space-y-0.5">
            <Label htmlFor="layout" className="text-sm font-medium">
              Default Layout
            </Label>
            <p className="text-xs text-muted-foreground">
              The layout used when no application-specific layout is set.
            </p>
          </div>
          <Select value={defaultLayout} onValueChange={(v) => { setDefaultLayout(v); handleUpdate({ defaultLayout: v }); }}>
            <SelectTrigger id="layout" className="h-8 w-full max-w-xs text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="us-qwerty">US QWERTY</SelectItem>
              <SelectItem value="uk-qwerty">UK QWERTY</SelectItem>
              <SelectItem value="de-qwertz">German QWERTZ</SelectItem>
              <SelectItem value="fr-azerty">French AZERTY</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="h-px bg-border" />

        <div className="space-y-3.5">
          <div className="space-y-0.5">
            <div className="flex items-center justify-between">
              <Label htmlFor="aggressiveness" className="text-sm font-medium">
                Correction Aggressiveness
              </Label>
              <span className="text-xs tabular-nums text-muted-foreground">
                {aggressiveness[0]}%
              </span>
            </div>
            <p className="text-xs text-muted-foreground">
              Higher values correct more aggressively.
            </p>
          </div>
          <Slider
            id="aggressiveness"
            value={aggressiveness}
            onValueChange={(v) => setAggressiveness(v)}
            onValueCommit={(v) => handleUpdate({ aggressiveness: v[0] })}
            max={100}
            step={1}
          />
        </div>
      </CardContent>
    </Card>
  );
}

function BlacklistSettings() {
  const [blacklist, setBlacklist] = useState<string[]>([]);
  const [newApp, setNewApp] = useState("");

  useEffect(() => {
    invoke<AppSettings>("get_settings")
      .then((s) => setBlacklist(s.blacklist))
      .catch(console.error);
  }, []);

  const handleUpdate = (updatedList: string[]) => {
    invoke<AppSettings>("get_settings").then((current) => {
      const updated = { ...current, blacklist: updatedList };
      return invoke("update_settings", { settings: updated });
    }).catch(console.error);
  };

  const addToBlacklist = () => {
    const trimmed = newApp.trim().toLowerCase();
    if (trimmed && !blacklist.includes(trimmed)) {
      const newList = [...blacklist, trimmed];
      setBlacklist(newList);
      handleUpdate(newList);
      setNewApp("");
    }
  };

  const removeFromBlacklist = (app: string) => {
    const newList = blacklist.filter((item) => item !== app);
    setBlacklist(newList);
    handleUpdate(newList);
  };

  return (
    <Card className="border border-border bg-card">
      <CardHeader className="pb-4">
        <CardTitle className="text-base font-medium">
          Application Blacklist
        </CardTitle>
        <CardDescription className="text-xs">
          Applications where AltShift should not apply corrections.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center gap-2">
          <Input
            placeholder="application.exe"
            value={newApp}
            onChange={(e) => setNewApp(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && addToBlacklist()}
            className="h-8 max-w-xs text-sm"
          />
          <Button
            onClick={addToBlacklist}
            size="sm"
            className="h-8 gap-1 text-xs"
          >
            <Plus className="h-3.5 w-3.5" />
            Add
          </Button>
        </div>

        <div className="overflow-hidden rounded-md border border-border">
          <Table>
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead className="h-8 text-xs font-medium">
                  Application
                </TableHead>
                <TableHead className="h-8 w-16 text-right text-xs font-medium">
                  Action
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {blacklist.map((app) => (
                <TableRow key={app} className="group">
                  <TableCell className="h-10 py-0 font-mono text-xs">
                    {app}
                  </TableCell>
                  <TableCell className="h-10 py-0 text-right">
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 opacity-0 group-hover:opacity-100"
                      onClick={() => removeFromBlacklist(app)}
                      aria-label={`Remove ${app}`}
                    >
                      <Trash2 className="h-3.5 w-3.5 text-destructive" />
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      </CardContent>
    </Card>
  );
}

function StatisticsSettings() {
  return (
    <div className="grid gap-4 sm:grid-cols-2">
      <Card className="border border-border bg-card">
        <CardHeader className="pb-2">
          <CardDescription className="text-xs">
            Total Corrections Made
          </CardDescription>
          <CardTitle className="text-2xl font-semibold tabular-nums">
            12,438
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-xs text-muted-foreground">
            Across all applications since install.
          </p>
        </CardContent>
      </Card>

      <Card className="border border-border bg-card">
        <CardHeader className="pb-2">
          <CardDescription className="text-xs">Time Saved</CardDescription>
          <CardTitle className="text-2xl font-semibold tabular-nums">
            4h 12m
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-xs text-muted-foreground">
            Estimated from corrected keystrokes.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}

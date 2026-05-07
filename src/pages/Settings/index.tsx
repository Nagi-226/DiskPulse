import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, RiskRule, RiskLevel } from "../../types";

const RISK_STYLES: Record<RiskLevel, string> = {
  low: "bg-risk-low-bg text-success border-success/20",
  medium: "bg-risk-medium-bg text-warning border-warning/20",
  high: "bg-risk-high-bg text-danger border-danger/20",
};

const POLL_PRESETS = [
  { label: "1s", value: 1000 },
  { label: "2s", value: 2000 },
  { label: "5s", value: 5000 },
  { label: "10s", value: 10000 },
];

const DEBOUNCE_PRESETS = [
  { label: "0.5s", value: 500 },
  { label: "1.5s", value: 1500 },
  { label: "3s", value: 3000 },
  { label: "5s", value: 5000 },
];

type SettingsTab = "general" | "rules" | "about";

// ─── Toggle Switch ──────────────────────────────────────────

function Toggle({
  checked,
  onChange,
  disabled,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors duration-200
        ${checked ? "bg-accent" : "bg-aurora-border"}
        ${disabled ? "opacity-50 cursor-not-allowed" : "cursor-pointer hover:opacity-90"}`}
    >
      <span
        className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform duration-200
          ${checked ? "translate-x-6" : "translate-x-1"}`}
      />
    </button>
  );
}

// ─── General Tab ────────────────────────────────────────────

function GeneralTab({
  settings,
  drives,
  saving,
  onUpdate,
  onSave,
  message,
}: {
  settings: AppSettings;
  drives: string[];
  saving: boolean;
  onUpdate: (s: AppSettings) => void;
  onSave: () => void;
  message: string | null;
}) {
  return (
    <div className="glass-card p-8 rounded-3xl border border-aurora-border/50 space-y-8">
      {/* Default Drive */}
      <div className="flex items-center justify-between py-1">
        <div>
          <div className="text-sm text-text-primary font-medium">默认驱动器</div>
          <p className="text-xs text-text-muted mt-1">启动时自动扫描的目标驱动器</p>
        </div>
        <select
          value={settings.default_drive}
          onChange={(e) => onUpdate({ ...settings, default_drive: e.target.value })}
          className="px-4 py-2.5 rounded-lg bg-aurora-elevated border border-aurora-border/50 text-sm text-text-primary
                     focus:outline-none focus:border-accent/50 appearance-none cursor-pointer"
          style={{
            backgroundImage: `url("data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' fill='none' viewBox='0 0 20 20'%3e%3cpath stroke='%2394a3b8' stroke-linecap='round' stroke-linejoin='round' stroke-width='1.5' d='M6 8l4 4 4-4'/%3e%3c/svg%3e")`,
            backgroundPosition: "right 10px center",
            backgroundRepeat: "no-repeat",
            backgroundSize: "14px",
            paddingRight: "34px",
          }}
        >
          {drives.map((d) => (
            <option key={d} value={d}>{d}: Drive</option>
          ))}
        </select>
      </div>

      <hr className="border-aurora-border/40" />

      {/* Auto Scan on Startup */}
      <div className="flex items-center justify-between py-1">
        <div>
          <div className="text-sm text-text-primary font-medium">启动时自动扫描</div>
          <p className="text-xs text-text-muted mt-1">应用启动后自动扫描默认驱动器</p>
        </div>
        <Toggle
          checked={settings.auto_scan_on_startup}
          onChange={(v) => onUpdate({ ...settings, auto_scan_on_startup: v })}
        />
      </div>

      <hr className="border-aurora-border/40" />

      {/* Auto Monitor on Startup */}
      <div className="flex items-center justify-between py-1">
        <div>
          <div className="text-sm text-text-primary font-medium">启动时自动监控</div>
          <p className="text-xs text-text-muted mt-1">应用启动后自动开启文件系统监控</p>
        </div>
        <Toggle
          checked={settings.auto_monitor_on_startup}
          onChange={(v) => onUpdate({ ...settings, auto_monitor_on_startup: v })}
        />
      </div>

      <hr className="border-aurora-border/40" />

      {/* Poll Interval */}
      <div>
        <div className="text-sm text-text-primary font-medium mb-4">监控轮询间隔</div>
        <div className="flex items-center gap-2">
          {POLL_PRESETS.map((p) => (
            <button
              key={p.value}
              className={`px-4 py-2 rounded-lg text-xs font-medium border transition-colors ${
                settings.watcher_poll_interval_ms === p.value
                  ? "bg-accent/15 border-accent/30 text-accent-light"
                  : "bg-aurora-elevated/70 border-aurora-border/60 text-text-secondary hover:text-text-primary"
              }`}
              onClick={() => onUpdate({ ...settings, watcher_poll_interval_ms: p.value })}
            >
              {p.label}
            </button>
          ))}
          <span className="ml-2 text-xs text-text-muted font-mono">
            {settings.watcher_poll_interval_ms}ms
          </span>
        </div>
      </div>

      {/* Debounce */}
      <div>
        <div className="text-sm text-text-primary font-medium mb-4">变更去抖窗口</div>
        <div className="flex items-center gap-2">
          {DEBOUNCE_PRESETS.map((p) => (
            <button
              key={p.value}
              className={`px-4 py-2 rounded-lg text-xs font-medium border transition-colors ${
                settings.watcher_debounce_ms === p.value
                  ? "bg-accent/15 border-accent/30 text-accent-light"
                  : "bg-aurora-elevated/70 border-aurora-border/60 text-text-secondary hover:text-text-primary"
              }`}
              onClick={() => onUpdate({ ...settings, watcher_debounce_ms: p.value })}
            >
              {p.label}
            </button>
          ))}
          <span className="ml-2 text-xs text-text-muted font-mono">
            {settings.watcher_debounce_ms}ms
          </span>
        </div>
      </div>

      {/* Save Button */}
      <div className="flex items-center gap-3 pt-4">
        <button className="btn-primary py-2.5 px-6" onClick={onSave} disabled={saving}>
          {saving ? "保存中..." : "保存设置"}
        </button>
        {message && (
          <span className={`text-xs ${message.startsWith("✓") ? "text-success" : "text-danger"}`}>
            {message}
          </span>
        )}
      </div>
    </div>
  );
}

// ─── Rules Tab ──────────────────────────────────────────────

function RulesTab() {
  const [rules, setRules] = useState<RiskRule[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<RiskLevel | "all">("all");
  const [expandedId, setExpandedId] = useState<string | null>(null);

  useEffect(() => {
    loadRules();
  }, []);

  async function loadRules() {
    setLoading(true);
    setError(null);
    try {
      const list = await invoke<RiskRule[]>("get_rules");
      setRules(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function handleToggle(ruleId: string, currentVal: boolean) {
    const newVal = !currentVal;
    // Optimistic update
    setRules((prev) =>
      prev.map((r) => (r.id === ruleId ? { ...r, safe_to_delete: newVal } : r))
    );
    try {
      await invoke("save_rule_override", { ruleId, safeToDelete: newVal });
    } catch (e) {
      setError(String(e));
      // Revert on failure
      setRules((prev) =>
        prev.map((r) => (r.id === ruleId ? { ...r, safe_to_delete: currentVal } : r))
      );
    }
  }

  const filtered = rules.filter((r) => {
    if (filter !== "all" && r.risk_level !== filter) return false;
    if (query.trim()) {
      const q = query.toLowerCase();
      return [r.id, r.category, r.explanation].some((v) => v.toLowerCase().includes(q));
    }
    return true;
  });

  return (
    <div className="glass-card p-8 rounded-3xl border border-aurora-border/50 space-y-5">
      {/* Controls */}
      <div className="flex flex-wrap items-center gap-4">
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="搜索规则名称或类别..."
          className="flex-1 min-w-48 rounded-xl bg-aurora-elevated/70 border border-aurora-border/60 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60"
        />
        <div className="flex gap-2">
          {(["all", "low", "medium", "high"] as const).map((f) => (
            <button
              key={f}
              className={`px-3.5 py-2 rounded-lg text-xs font-medium border transition-colors ${
                filter === f
                  ? f === "all"
                    ? "bg-accent/15 border-accent/30 text-accent-light"
                    : `${RISK_STYLES[f]}`
                  : "bg-aurora-elevated/70 border-aurora-border/60 text-text-secondary hover:text-text-primary"
              }`}
              onClick={() => setFilter(f)}
            >
              {f === "all" ? "全部" : f === "low" ? "低" : f === "medium" ? "中" : "高"}
            </button>
          ))}
        </div>
        <button
          className="px-3.5 py-2 rounded-lg text-xs border bg-aurora-elevated/70 border-aurora-border/60 text-text-secondary hover:text-accent-light"
          onClick={loadRules}
        >
          刷新
        </button>
      </div>

      {error && (
        <div className="p-3 rounded-xl bg-risk-high-bg/20 border border-red-500/20 text-sm text-danger">
          {error}
        </div>
      )}

      {loading ? (
        <div className="flex items-center justify-center py-16">
          <svg className="animate-spin" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" style={{ color: "var(--color-accent)" }}>
            <circle cx="12" cy="12" r="10" strokeDasharray="32" strokeDashoffset="32" />
          </svg>
          <span className="ml-3 text-sm text-text-muted">加载规则...</span>
        </div>
      ) : (
        <div className="max-h-[55vh] overflow-y-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-xs text-text-muted uppercase tracking-wider border-b border-aurora-border/40">
                <th className="text-left px-4 py-3 font-medium">规则 ID</th>
                <th className="text-left px-4 py-3 font-medium">类别</th>
                <th className="text-left px-4 py-3 font-medium">风险等级</th>
                <th className="text-center px-4 py-3 font-medium w-24">可安全删除</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((rule) => {
                const isExpanded = expandedId === rule.id;
                return (
                  <>
                    <tr
                      key={rule.id}
                      className={`border-b border-aurora-border/20 cursor-pointer transition-colors hover:bg-aurora-elevated/40 ${
                        isExpanded ? "bg-aurora-elevated/30" : ""
                      }`}
                      onClick={() => setExpandedId(isExpanded ? null : rule.id)}
                    >
                      <td className="px-4 py-3 font-mono text-xs text-text-primary">{rule.id}</td>
                      <td className="px-4 py-3 text-text-secondary">{rule.category}</td>
                      <td className="px-4 py-3">
                        <span
                          className={`inline-block px-2.5 py-1 rounded-full text-xs font-medium border ${
                            RISK_STYLES[rule.risk_level]
                          }`}
                        >
                          {rule.risk_level === "low" ? "低" : rule.risk_level === "medium" ? "中" : "高"}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-center">
                        <Toggle
                          checked={rule.safe_to_delete}
                          onChange={() => handleToggle(rule.id, rule.safe_to_delete)}
                        />
                      </td>
                    </tr>
                    {isExpanded && (
                      <tr key={`${rule.id}-detail`} className="border-b border-aurora-border/20">
                        <td colSpan={4} className="px-6 py-5 bg-aurora-elevated/20">
                          <div className="space-y-4">
                            <div>
                              <span className="text-xs text-text-muted">匹配模式：</span>
                              <span className="text-xs text-text-primary font-mono ml-1">
                                {rule.patterns.join(", ")}
                              </span>
                            </div>
                            {rule.name_match && (
                              <div>
                                <span className="text-xs text-text-muted">名称匹配：</span>
                                <span className="text-xs text-text-primary font-mono ml-1">
                                  {rule.name_match}
                                </span>
                              </div>
                            )}
                            <p className="text-sm text-text-secondary leading-6">
                              {rule.explanation}
                            </p>
                          </div>
                        </td>
                      </tr>
                    )}
                  </>
                );
              })}
            </tbody>
          </table>
          {filtered.length === 0 && (
            <div className="text-center py-12 text-text-muted text-sm">
              没有匹配的规则
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ─── About Tab ──────────────────────────────────────────────

function AboutTab() {
  const [version, setVersion] = useState("");

  useEffect(() => {
    invoke<string>("app_version").then(setVersion);
  }, []);

  const techStack = [
    {
      name: "Tauri 2",
      desc: "Rust 驱动的轻量级桌面框架",
      color: "#6366f1",
    },
    {
      name: "React 19",
      desc: "TypeScript 前端用户界面",
      color: "#06b6d4",
    },
    {
      name: "SQLite",
      desc: "本地历史记录与设置存储",
      color: "#10b981",
    },
    {
      name: "ECharts",
      desc: "交互式数据可视化图表",
      color: "#f59e0b",
    },
  ];

  return (
    <div className="glass-card p-10 rounded-3xl border border-aurora-border/50">
      {/* Logo + Name */}
      <div className="flex flex-col items-center pt-8 pb-10">
        <div
          className="w-20 h-20 rounded-2xl flex items-center justify-center mb-5"
          style={{
            background: "linear-gradient(135deg, var(--color-accent), #7c3aed)",
            boxShadow: "0 8px 30px var(--color-accent-glow)",
          }}
        >
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="white" strokeWidth="2" strokeLinecap="round">
            <circle cx="12" cy="12" r="10" />
            <circle cx="12" cy="12" r="4" />
            <path d="M12 2v4M12 18v4M2 12h4M18 12h4" />
          </svg>
        </div>
        <h2 className="text-3xl font-bold text-text-primary">DiskPulse</h2>
        <p className="text-sm text-text-muted mt-2 font-mono">
          v{version || "0.1.0"}
        </p>
        <p className="text-sm text-text-secondary mt-4 text-center max-w-xs leading-6">
          实时磁盘空间监控与安全清理工具
          <br />
          专为 Windows 11 设计
        </p>
      </div>

      {/* Tech Stack Grid */}
      <div className="grid grid-cols-2 gap-4 mb-8">
        {techStack.map((tech) => (
          <div
            key={tech.name}
            className="rounded-2xl bg-aurora-elevated/60 border border-aurora-border/40 p-5"
          >
            <div className="flex items-center gap-3 mb-3">
              <span
                className="w-2.5 h-2.5 rounded-full flex-shrink-0"
                style={{ backgroundColor: tech.color }}
              />
              <span className="text-sm font-semibold text-text-primary">{tech.name}</span>
            </div>
            <p className="text-xs text-text-muted leading-5">{tech.desc}</p>
          </div>
        ))}
      </div>

      {/* Footer */}
      <div className="pt-5 border-t border-aurora-border/40 text-center">
        <p className="text-sm text-text-muted">
          Built by FJL03 &nbsp;|&nbsp; MIT License &nbsp;|&nbsp; © 2026
        </p>
      </div>
    </div>
  );
}

// ─── Main Settings Page ─────────────────────────────────────

export default function SettingsPage() {
  const [tab, setTab] = useState<SettingsTab>("general");
  const [settings, setSettings] = useState<AppSettings>({
    default_drive: "C",
    auto_scan_on_startup: false,
    auto_monitor_on_startup: false,
    watcher_poll_interval_ms: 2000,
    watcher_debounce_ms: 1500,
  });
  const [drives, setDrives] = useState<string[]>(["C"]);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    invoke<AppSettings>("get_settings")
      .then(setSettings)
      .catch((e) => console.error("get_settings:", e));
    invoke<string[]>("list_drives")
      .then((list) => {
        setDrives(list);
        if (list.length > 0 && !list.includes("C")) {
          setSettings((prev) => ({ ...prev, default_drive: list[0] }));
        }
      })
      .catch((e) => console.error("list_drives:", e));
  }, []);

  async function handleSave() {
    setSaving(true);
    setMessage(null);
    try {
      await invoke("save_settings", { settings });
      setMessage("✓ 设置已保存");
      setTimeout(() => setMessage(null), 3000);
    } catch (e) {
      setMessage(`✗ 保存失败: ${String(e)}`);
    } finally {
      setSaving(false);
    }
  }

  const TABS: { id: SettingsTab; label: string }[] = [
    { id: "general", label: "通用" },
    { id: "rules", label: "规则" },
    { id: "about", label: "关于" },
  ];

  return (
    <div className="p-8 space-y-6">
      {/* Header */}
      <div>
        <h2 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
          设置
        </h2>
        <p className="text-xs text-text-muted mt-1">偏好设置、规则配置与应用信息</p>
      </div>

      {/* Tab Bar */}
      <div className="flex gap-1 rounded-xl bg-aurora-elevated/70 border border-aurora-border/50 p-1 w-fit">
        {TABS.map((t) => (
          <button
            key={t.id}
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
              tab === t.id
                ? "bg-accent/20 text-accent-light"
                : "text-text-secondary hover:text-text-primary"
            }`}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Tab Content */}
      {tab === "general" && (
        <GeneralTab
          settings={settings}
          drives={drives}
          saving={saving}
          onUpdate={setSettings}
          onSave={handleSave}
          message={message}
        />
      )}
      {tab === "rules" && <RulesTab />}
      {tab === "about" && <AboutTab />}
    </div>
  );
}

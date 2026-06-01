import { Fragment, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { applyLanguage, LANGUAGE_OPTIONS } from "../../i18n";
import { THEME_OPTIONS, useTheme, type ThemeId } from "../../hooks/useTheme";
import type { AppSettings, AutoCleanupStatus, CleanResult, RiskLevel, RiskRule } from "../../types";
import { formatSize } from "../../utils/format";

type SettingsTab = "general" | "appearance" | "rules" | "alerts" | "automation" | "about";

const RISK_STYLES: Record<RiskLevel, string> = {
  low: "bg-risk-low-bg text-success border-success/20",
  medium: "bg-risk-medium-bg text-warning border-warning/20",
  high: "bg-risk-high-bg text-danger border-danger/20",
};

const DEFAULT_SETTINGS: AppSettings = {
  default_drive: "C",
  auto_scan_on_startup: false,
  auto_monitor_on_startup: false,
  watcher_poll_interval_ms: 2000,
  watcher_debounce_ms: 1500,
  alert_enabled: false,
  alert_threshold_type: "percentage",
  alert_threshold_value: 10,
  alert_growth_enabled: false,
  alert_growth_percent: 5,
  alert_growth_minutes: 15,
  auto_cleanup_enabled: false,
  auto_cleanup_frequency: "weekly",
  auto_cleanup_time: "03:00",
  auto_cleanup_risk_levels: "low",
  auto_cleanup_min_free_gb: 50,
  language: "auto",
  theme: "auto",
};

function Toggle({ checked, onChange, disabled }: { checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors duration-200 ${
        checked ? "bg-accent" : "bg-aurora-border"
      } ${disabled ? "cursor-not-allowed opacity-50" : "cursor-pointer hover:opacity-90"}`}
    >
      <span className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform duration-200 ${checked ? "translate-x-6" : "translate-x-1"}`} />
    </button>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="space-y-2">
      <span className="block text-xs font-medium uppercase tracking-wider text-text-muted">{label}</span>
      {children}
    </label>
  );
}

type SaveHandler = () => Promise<boolean>;

function SaveRow({ saving, message, onSave, label = "Save Settings" }: { saving: boolean; message: string | null; onSave: SaveHandler; label?: string }) {
  return (
    <div className="flex items-center gap-3 pt-4">
      <button className="btn-primary py-2.5 px-6" onClick={() => void onSave()} disabled={saving}>
        <span>{saving ? "Saving..." : label}</span>
      </button>
      {message && <span className={`text-xs ${message.startsWith("Saved") ? "text-success" : "text-danger"}`}>{message}</span>}
    </div>
  );
}

function GeneralTab({ settings, drives, saving, onUpdate, onSave, message }: {
  settings: AppSettings;
  drives: string[];
  saving: boolean;
  onUpdate: (s: AppSettings) => void;
  onSave: SaveHandler;
  message: string | null;
}) {
  const pollPresets = [1000, 2000, 5000, 10000];
  const debouncePresets = [500, 1500, 3000, 5000];
  const { t } = useTranslation();

  return (
    <div className="glass-card p-6 rounded-2xl border border-aurora-border/50 space-y-6">
      <div className="flex items-center justify-between gap-4 py-3">
        <div>
          <div className="text-sm font-medium text-text-primary">Default drive</div>
          <p className="mt-1 text-xs text-text-muted">Drive used for startup scans and scheduled jobs.</p>
        </div>
        <select
          value={settings.default_drive}
          onChange={(e) => onUpdate({ ...settings, default_drive: e.target.value })}
          className="rounded-lg border border-aurora-border/50 bg-aurora-elevated px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/50"
        >
          {drives.map((drive) => (
            <option key={drive} value={drive}>{drive}: Drive</option>
          ))}
        </select>
      </div>

      <hr className="border-aurora-border/40" />

      <div className="flex items-center justify-between gap-4 py-3">
        <div>
          <div className="text-sm font-medium text-text-primary">{t("settings.language")}</div>
          <p className="mt-1 text-xs text-text-muted">{t("settings.languageHelp")}</p>
        </div>
        <select
          value={settings.language}
          onChange={(e) => {
            const language = e.target.value;
            applyLanguage(language);
            onUpdate({ ...settings, language });
          }}
          className="rounded-lg border border-aurora-border/50 bg-aurora-elevated px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/50"
        >
          {LANGUAGE_OPTIONS.map((option) => (
            <option key={option.id} value={option.id}>{t(option.labelKey)}</option>
          ))}
        </select>
      </div>

      <hr className="border-aurora-border/40" />

      <ToggleRow title="Auto scan on startup" detail="Scan the default drive when DiskPulse starts." checked={settings.auto_scan_on_startup} onChange={(v) => onUpdate({ ...settings, auto_scan_on_startup: v })} />
      <ToggleRow title="Auto monitor on startup" detail="Start file-system monitoring when DiskPulse starts." checked={settings.auto_monitor_on_startup} onChange={(v) => onUpdate({ ...settings, auto_monitor_on_startup: v })} />

      <PresetRow title="Watcher poll interval" value={settings.watcher_poll_interval_ms} presets={pollPresets} onChange={(v) => onUpdate({ ...settings, watcher_poll_interval_ms: v })} />
      <PresetRow title="Debounce window" value={settings.watcher_debounce_ms} presets={debouncePresets} onChange={(v) => onUpdate({ ...settings, watcher_debounce_ms: v })} />

      <SaveRow saving={saving} message={message} onSave={onSave} />
    </div>
  );
}

function AppearanceTab({ settings, saving, onUpdate, onSave, message }: {
  settings: AppSettings;
  saving: boolean;
  onUpdate: (s: AppSettings) => void;
  onSave: SaveHandler;
  message: string | null;
}) {
  const { t } = useTranslation();
  const { setTheme, resolvedTheme } = useTheme();

  function handleThemeChange(theme: string) {
    const nextTheme = (theme === "light" || theme === "dark" || theme === "auto" ? theme : "auto") as ThemeId;
    setTheme(nextTheme);
    onUpdate({ ...settings, theme: nextTheme });
  }

  return (
    <div className="glass-card p-6 rounded-2xl border border-aurora-border/50 space-y-6">
      <div className="flex items-center justify-between gap-4 py-3">
        <div>
          <div className="text-sm font-medium text-text-primary">{t("settings.theme")}</div>
          <p className="mt-1 text-xs text-text-muted">{t("settings.themeHelp")}</p>
        </div>
        <select
          value={settings.theme}
          onChange={(e) => handleThemeChange(e.target.value)}
          className="rounded-lg border border-aurora-border/50 bg-aurora-elevated px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/50"
        >
          {THEME_OPTIONS.map((option) => (
            <option key={option.id} value={option.id}>{t(option.labelKey)}</option>
          ))}
        </select>
      </div>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <div className="rounded-2xl border border-aurora-border/40 bg-aurora-elevated/60 p-5">
          <div className="text-xs uppercase tracking-wider text-text-muted">Resolved theme</div>
          <div className="mt-2 text-lg font-semibold text-text-primary">{resolvedTheme}</div>
        </div>
        <div className="rounded-2xl border border-accent/20 bg-accent/10 p-5">
          <div className="text-xs uppercase tracking-wider text-text-muted">Token system</div>
          <div className="mt-2 text-sm text-text-secondary">CSS variables drive all Aurora surfaces and text colors.</div>
        </div>
      </div>

      <SaveRow saving={saving} message={message} onSave={onSave} />
    </div>
  );
}

function ToggleRow({ title, detail, checked, onChange }: { title: string; detail: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <div className="flex items-center justify-between gap-4 py-3">
      <div>
        <div className="text-sm font-medium text-text-primary">{title}</div>
        <p className="mt-1 text-xs text-text-muted">{detail}</p>
      </div>
      <Toggle checked={checked} onChange={onChange} />
    </div>
  );
}

function PresetRow({ title, value, presets, onChange }: { title: string; value: number; presets: number[]; onChange: (v: number) => void }) {
  return (
    <div>
      <div className="mb-4 text-sm font-medium text-text-primary">{title}</div>
      <div className="flex flex-wrap items-center gap-2">
        {presets.map((preset) => (
          <button
            key={preset}
            className={`rounded-lg border px-4 py-2 text-xs font-medium transition-colors ${
              value === preset
                ? "border-accent/30 bg-accent/15 text-accent-light"
                : "border-aurora-border/60 bg-aurora-elevated/70 text-text-secondary hover:text-text-primary"
            }`}
            onClick={() => onChange(preset)}
          >
            {preset >= 1000 ? `${preset / 1000}s` : `${preset}ms`}
          </button>
        ))}
        <span className="ml-2 font-mono text-xs text-text-muted">{value}ms</span>
      </div>
    </div>
  );
}

function RulesTab() {
  const [rules, setRules] = useState<RiskRule[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<RiskLevel | "all">("all");
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [customName, setCustomName] = useState("");
  const [customPattern, setCustomPattern] = useState("");
  const [customRisk, setCustomRisk] = useState<RiskLevel>("medium");

  async function loadRules() {
    setLoading(true);
    setError(null);
    try {
      setRules(await invoke<RiskRule[]>("get_rules"));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    loadRules();
  }, []);

  async function handleToggle(ruleId: string, currentValue: boolean) {
    const nextValue = !currentValue;
    setRules((prev) => prev.map((rule) => (rule.id === ruleId ? { ...rule, safe_to_delete: nextValue } : rule)));
    try {
      await invoke("save_rule_override", { ruleId, safeToDelete: nextValue });
    } catch (e) {
      setError(String(e));
      setRules((prev) => prev.map((rule) => (rule.id === ruleId ? { ...rule, safe_to_delete: currentValue } : rule)));
    }
  }

  async function handleCreateCustomRule() {
    if (!customName.trim() || !customPattern.trim()) {
      setError("Custom rule requires a name and pattern.");
      return;
    }
    setError(null);
    try {
      await invoke<RiskRule>("create_custom_rule", {
        name: customName,
        pattern: customPattern,
        riskLevel: customRisk,
      });
      setCustomName("");
      setCustomPattern("");
      await loadRules();
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleDeleteCustomRule(ruleId: string) {
    try {
      await invoke("delete_custom_rule", { ruleId });
      await loadRules();
    } catch (e) {
      setError(String(e));
    }
  }

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return rules.filter((rule) => {
      if (filter !== "all" && rule.risk_level !== filter) return false;
      if (!q) return true;
      return [rule.id, rule.category, rule.explanation].some((value) => value.toLowerCase().includes(q));
    });
  }, [filter, query, rules]);

  return (
    <div className="glass-card p-6 rounded-2xl border border-aurora-border/50 space-y-5">
      <div className="flex flex-wrap items-center gap-4">
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search rules"
          className="min-w-48 flex-1 rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60"
        />
        <div className="flex gap-2">
          {(["all", "low", "medium", "high"] as const).map((item) => (
            <button
              key={item}
              className={`rounded-lg border px-3.5 py-2 text-xs font-medium transition-colors ${
                filter === item
                  ? item === "all"
                    ? "border-accent/30 bg-accent/15 text-accent-light"
                    : RISK_STYLES[item]
                  : "border-aurora-border/60 bg-aurora-elevated/70 text-text-secondary hover:text-text-primary"
              }`}
              onClick={() => setFilter(item)}
            >
              {item}
            </button>
          ))}
        </div>
        <button className="rounded-lg border border-aurora-border/60 bg-aurora-elevated/70 px-3.5 py-2 text-xs text-text-secondary hover:text-accent-light" onClick={loadRules}>
          Refresh
        </button>
      </div>

      <div className="rounded-2xl border border-aurora-border/40 bg-aurora-elevated/40 p-4">
        <div className="mb-3 text-sm font-semibold text-text-primary">Custom rule</div>
        <div className="grid grid-cols-1 gap-3 md:grid-cols-[1fr_1fr_150px_auto]">
          <input value={customName} onChange={(e) => setCustomName(e.target.value)} placeholder="Rule name" className="rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60" />
          <input value={customPattern} onChange={(e) => setCustomPattern(e.target.value)} placeholder="Path pattern, e.g. archive-cache" className="rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60" />
          <select value={customRisk} onChange={(e) => setCustomRisk(e.target.value as RiskLevel)} className="rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60">
            <option value="low">low</option>
            <option value="medium">medium</option>
            <option value="high">high</option>
          </select>
          <button className="rounded-xl border border-accent/30 bg-accent/15 px-4 py-2.5 text-sm font-semibold text-accent-light" onClick={() => void handleCreateCustomRule()}>Create</button>
        </div>
        <p className="mt-2 text-xs text-text-muted">Custom rules are review-only by default; they do not become safe-to-delete automatically.</p>
      </div>

      {error && <div className="rounded-xl border border-red-500/20 bg-risk-high-bg/20 p-3 text-sm text-danger">{error}</div>}

      {loading ? (
        <div className="py-24 text-center text-sm text-text-muted">Loading rules...</div>
      ) : (
        <div className="max-h-[55vh] overflow-y-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-aurora-border/40 text-xs uppercase tracking-wider text-text-muted">
                <th className="px-4 py-3 text-left font-medium">Rule ID</th>
                <th className="px-4 py-3 text-left font-medium">Category</th>
                <th className="px-4 py-3 text-left font-medium">Risk</th>
                <th className="w-24 px-4 py-3 text-center font-medium">Safe</th>
                <th className="w-24 px-4 py-3 text-right font-medium">Action</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((rule) => {
                const expanded = expandedId === rule.id;
                return (
                  <Fragment key={rule.id}>
                    <tr className="cursor-pointer border-b border-aurora-border/20 transition-colors hover:bg-aurora-elevated/40" onClick={() => setExpandedId(expanded ? null : rule.id)}>
                      <td className="px-4 py-3 font-mono text-xs text-text-primary">{rule.id}</td>
                      <td className="px-4 py-3 text-text-secondary">{rule.category}</td>
                      <td className="px-4 py-3"><span className={`rounded-full border px-2.5 py-1 text-xs font-medium ${RISK_STYLES[rule.risk_level]}`}>{rule.risk_level}</span></td>
                      <td className="px-4 py-3 text-center"><Toggle checked={rule.safe_to_delete} onChange={() => handleToggle(rule.id, rule.safe_to_delete)} /></td>
                      <td className="px-4 py-3 text-right">
                        {rule.id.startsWith("custom-") && (
                          <button className="text-xs text-danger hover:text-text-primary" onClick={(e) => { e.stopPropagation(); void handleDeleteCustomRule(rule.id); }}>Delete</button>
                        )}
                      </td>
                    </tr>
                    {expanded && (
                      <tr key={`${rule.id}-detail`} className="border-b border-aurora-border/20">
                        <td colSpan={5} className="bg-aurora-elevated/20 px-6 py-5">
                          <div className="space-y-3 text-sm text-text-secondary">
                            <div><span className="text-text-muted">Patterns:</span> <span className="font-mono text-text-primary">{rule.patterns.join(", ")}</span></div>
                            {rule.name_match && <div><span className="text-text-muted">Name match:</span> <span className="font-mono text-text-primary">{rule.name_match}</span></div>}
                            <p className="leading-6">{rule.explanation}</p>
                          </div>
                        </td>
                      </tr>
                    )}
                  </Fragment>
                );
              })}
            </tbody>
          </table>
          {filtered.length === 0 && <div className="py-24 text-center text-sm text-text-muted">No matching rules.</div>}
        </div>
      )}
    </div>
  );
}

function AlertsTab({ settings, saving, onUpdate, onSave, message }: {
  settings: AppSettings;
  saving: boolean;
  onUpdate: (s: AppSettings) => void;
  onSave: SaveHandler;
  message: string | null;
}) {
  return (
    <div className="glass-card p-6 rounded-2xl border border-aurora-border/50 space-y-6">
      <ToggleRow title="Disk space alerts" detail="Send notifications when free space is below the configured threshold." checked={settings.alert_enabled} onChange={(v) => onUpdate({ ...settings, alert_enabled: v })} />

      {settings.alert_enabled && (
        <>
          <hr className="border-aurora-border/40" />
          <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
            <Field label="Threshold type">
              <select value={settings.alert_threshold_type} onChange={(e) => onUpdate({ ...settings, alert_threshold_type: e.target.value })} className="w-full rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60">
                <option value="percentage">Free percentage</option>
                <option value="absolute_gb">Free GB</option>
              </select>
            </Field>
            <Field label="Threshold value">
              <input type="number" value={settings.alert_threshold_value} onChange={(e) => onUpdate({ ...settings, alert_threshold_value: Number(e.target.value) })} className="w-full rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60" />
            </Field>
            <Field label="Growth window minutes">
              <input type="number" value={settings.alert_growth_minutes} onChange={(e) => onUpdate({ ...settings, alert_growth_minutes: Number(e.target.value) })} className="w-full rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60" />
            </Field>
          </div>
          <ToggleRow title="Sudden growth detection" detail="Warn when used space grows quickly within the configured window." checked={settings.alert_growth_enabled} onChange={(v) => onUpdate({ ...settings, alert_growth_enabled: v })} />
          {settings.alert_growth_enabled && (
            <Field label="Growth percent">
              <input type="number" value={settings.alert_growth_percent} onChange={(e) => onUpdate({ ...settings, alert_growth_percent: Number(e.target.value) })} className="w-40 rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60" />
            </Field>
          )}
        </>
      )}

      <SaveRow saving={saving} message={message} onSave={onSave} />
    </div>
  );
}

function AutomationTab({ settings, saving, onUpdate, onSave, message }: {
  settings: AppSettings;
  saving: boolean;
  onUpdate: (s: AppSettings) => void;
  onSave: SaveHandler;
  message: string | null;
}) {
  const [running, setRunning] = useState(false);
  const [runMessage, setRunMessage] = useState<string | null>(null);
  const [status, setStatus] = useState<AutoCleanupStatus | null>(null);

  useEffect(() => {
    invoke<AutoCleanupStatus>("get_auto_cleanup_status").then(setStatus).catch((e) => setRunMessage(String(e)));
  }, []);

  async function refreshStatus() {
    setStatus(await invoke<AutoCleanupStatus>("get_auto_cleanup_status"));
  }

  async function handleSaveAutomation() {
    const saved = await onSave();
    if (!saved) return;

    try {
      await refreshStatus();
      setRunMessage("Scheduler updated.");
      setTimeout(() => setRunMessage(null), 3000);
    } catch (e) {
      setRunMessage(`Status refresh failed: ${String(e)}`);
    }
  }

  async function handleRunNow() {
    if (!window.confirm("Run auto-cleanup now? Only LOW-risk, whitelisted items are eligible and deletion still goes to Recycle Bin.")) return;
    setRunning(true);
    setRunMessage(null);
    try {
      const result = await invoke<CleanResult>("run_auto_cleanup_now");
      setRunMessage(`Run complete: ${result.succeeded} cleaned, ${result.skipped} skipped, ${result.failed} failed.`);
      await refreshStatus();
    } catch (e) {
      setRunMessage(`Run failed: ${String(e)}`);
    } finally {
      setRunning(false);
    }
  }

  return (
    <div className="glass-card p-6 rounded-2xl border border-aurora-border/50 space-y-6">
      <ToggleRow title="Scheduled auto-cleanup" detail="Runs the existing safe cleanup pipeline for LOW-risk candidates only." checked={settings.auto_cleanup_enabled} onChange={(v) => onUpdate({ ...settings, auto_cleanup_enabled: v })} />

      <hr className="border-aurora-border/40" />

      <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
        <Field label="Frequency">
          <select value={settings.auto_cleanup_frequency} onChange={(e) => onUpdate({ ...settings, auto_cleanup_frequency: e.target.value })} className="w-full rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60">
            <option value="daily">Daily</option>
            <option value="weekly">Weekly</option>
            <option value="monthly">Monthly</option>
          </select>
        </Field>
        <Field label="Time">
          <input type="time" value={settings.auto_cleanup_time} onChange={(e) => onUpdate({ ...settings, auto_cleanup_time: e.target.value })} className="w-full rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60" />
        </Field>
        <Field label="Skip if free space is above">
          <div className="flex items-center gap-2">
            <input type="number" min={0} step={5} value={settings.auto_cleanup_min_free_gb} onChange={(e) => onUpdate({ ...settings, auto_cleanup_min_free_gb: Number(e.target.value) })} className="w-full rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60" />
            <span className="text-xs text-text-muted">GB</span>
          </div>
        </Field>
      </div>

      <div className="rounded-2xl border border-success/15 bg-risk-low-bg/10 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div className="text-sm font-semibold text-success">Eligible risk levels</div>
            <p className="mt-1 text-xs text-text-muted">Auto-cleanup is locked to LOW risk only. Medium and High risk items are never cleaned automatically.</p>
          </div>
          <span className="rounded-full border border-success/20 bg-risk-low-bg px-3 py-1 text-xs font-semibold text-success">LOW only</span>
        </div>
      </div>

      {status && (
        <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
          <StatusTile label="State" value={status.enabled ? "Enabled" : "Disabled"} />
          <StatusTile label="Next run" value={status.next_run_epoch_ms ? new Date(status.next_run_epoch_ms).toLocaleString() : "Not scheduled"} />
          <StatusTile label="Last freed" value={formatSize(status.last_freed_bytes)} />
        </div>
      )}

      <div className="flex flex-wrap items-center gap-3 pt-4">
        <button className="btn-primary py-2.5 px-6" onClick={() => void handleSaveAutomation()} disabled={saving}><span>{saving ? "Saving..." : "Save Automation"}</span></button>
        <button className="rounded-xl border border-warning/25 bg-risk-medium-bg px-4 py-2.5 text-sm font-semibold text-warning transition-colors hover:bg-risk-medium-bg/80" onClick={handleRunNow} disabled={running}>{running ? "Running..." : "Run Now"}</button>
        {message && <span className={`text-xs ${message.startsWith("Saved") ? "text-success" : "text-danger"}`}>{message}</span>}
        {runMessage && <span className={`text-xs ${runMessage.includes("failed") ? "text-danger" : "text-success"}`}>{runMessage}</span>}
      </div>

      <p className="text-xs text-text-muted">Note: scheduler changes are applied immediately after saving. Run Now uses the latest saved settings.</p>
    </div>
  );
}

function StatusTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl border border-aurora-border/40 bg-aurora-elevated/60 p-4">
      <div className="text-xs text-text-muted">{label}</div>
      <div className="mt-2 text-sm font-semibold text-text-primary">{value}</div>
    </div>
  );
}

function AboutTab() {
  const [version, setVersion] = useState("");
  useEffect(() => {
    invoke<string>("app_version").then(setVersion);
  }, []);

  return (
    <div className="glass-card p-8 rounded-2xl border border-aurora-border/50">
      <div className="flex flex-col items-center py-8 text-center">
        <div className="mb-5 flex h-20 w-20 items-center justify-center rounded-2xl bg-accent/20 text-3xl text-accent-light">DP</div>
        <h2 className="text-3xl font-bold text-text-primary">DiskPulse</h2>
        <p className="mt-2 font-mono text-sm text-text-muted">v{version || "0.3.0"}</p>
        <p className="mt-4 max-w-sm text-sm leading-6 text-text-secondary">Real-time disk space monitoring, risk classification, and safe cleanup for Windows 11.</p>
      </div>
      <div className="grid grid-cols-2 gap-4">
        {[
          ["Tauri 2", "Rust-powered desktop shell"],
          ["React 19", "TypeScript UI"],
          ["SQLite", "Local history and settings"],
          ["ECharts", "Interactive data visualization"],
        ].map(([name, desc]) => (
          <div key={name} className="rounded-2xl border border-aurora-border/40 bg-aurora-elevated/60 p-5">
            <div className="text-sm font-semibold text-text-primary">{name}</div>
            <p className="mt-2 text-xs leading-5 text-text-muted">{desc}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

export default function SettingsPage() {
  const { t } = useTranslation();
  const { setTheme } = useTheme();
  const [tab, setTab] = useState<SettingsTab>("general");
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [drives, setDrives] = useState<string[]>(["C"]);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    invoke<AppSettings>("get_settings").then((loaded) => {
      const merged = { ...DEFAULT_SETTINGS, ...loaded };
      setSettings(merged);
      applyLanguage(merged.language);
      setTheme((merged.theme === "light" || merged.theme === "dark" || merged.theme === "auto" ? merged.theme : "auto") as ThemeId);
    }).catch((e) => console.error("get_settings:", e));
    invoke<string[]>("list_drives").then((list) => {
      setDrives(list);
      if (list.length > 0 && !list.includes("C")) {
        setSettings((prev) => ({ ...prev, default_drive: list[0] }));
      }
    }).catch(() => setDrives(["C"]));
  }, []);

  async function handleSave(): Promise<boolean> {
    setSaving(true);
    setMessage(null);
    try {
      await invoke("save_settings", { settings });
      setMessage("Saved settings.");
      setTimeout(() => setMessage(null), 3000);
      return true;
    } catch (e) {
      setMessage(`Save failed: ${String(e)}`);
      return false;
    } finally {
      setSaving(false);
    }
  }

  const tabs: { id: SettingsTab; label: string }[] = [
    { id: "general", label: "General" },
    { id: "appearance", label: t("settings.appearance") },
    { id: "rules", label: "Rules" },
    { id: "alerts", label: "Alerts" },
    { id: "automation", label: "Automation" },
    { id: "about", label: "About" },
  ];

  return (
    <div className="p-8 space-y-6">
      <div>
        <h2 className="text-sm font-semibold uppercase tracking-wider text-text-primary">Settings</h2>
        <p className="mt-1 text-xs text-text-muted">Preferences, safety rules, alerts, and automation.</p>
      </div>

      <div className="flex w-fit gap-1 rounded-xl border border-aurora-border/50 bg-aurora-elevated/70 p-1">
        {tabs.map((item) => (
          <button key={item.id} className={`rounded-lg px-4 py-2 text-sm font-medium transition-colors ${tab === item.id ? "bg-accent/20 text-accent-light" : "text-text-secondary hover:text-text-primary"}`} onClick={() => setTab(item.id)}>
            {item.label}
          </button>
        ))}
      </div>

      {tab === "general" && <GeneralTab settings={settings} drives={drives} saving={saving} onUpdate={setSettings} onSave={handleSave} message={message} />}
      {tab === "appearance" && <AppearanceTab settings={settings} saving={saving} onUpdate={setSettings} onSave={handleSave} message={message} />}
      {tab === "rules" && <RulesTab />}
      {tab === "alerts" && <AlertsTab settings={settings} saving={saving} onUpdate={setSettings} onSave={handleSave} message={message} />}
      {tab === "automation" && <AutomationTab settings={settings} saving={saving} onUpdate={setSettings} onSave={handleSave} message={message} />}
      {tab === "about" && <AboutTab />}
    </div>
  );
}

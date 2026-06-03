import { useEffect, useState } from "react";
import type { RiskLevel } from "../types";
import RuleTester from "./RuleTester";

export interface RuleEditorValue {
  name: string;
  pattern: string;
  risk_level: Extract<RiskLevel, "low" | "medium">;
}

interface RuleEditorProps {
  initial?: RuleEditorValue;
  saving?: boolean;
  onCancel: () => void;
  onSave: (value: RuleEditorValue) => Promise<void>;
}

const EMPTY_RULE: RuleEditorValue = {
  name: "",
  pattern: "",
  risk_level: "medium",
};

export default function RuleEditor({ initial, saving = false, onCancel, onSave }: RuleEditorProps) {
  const [value, setValue] = useState<RuleEditorValue>(initial ?? EMPTY_RULE);
  const canSave = value.name.trim().length > 0 && value.pattern.trim().length > 0;

  useEffect(() => {
    setValue(initial ?? EMPTY_RULE);
  }, [initial]);

  return (
    <div className="rounded-2xl border border-accent/20 bg-accent/10 p-5">
      <div className="mb-4 flex items-start justify-between gap-4">
        <div>
          <div className="text-sm font-semibold text-text-primary">{initial ? "Edit custom rule" : "New custom rule"}</div>
          <p className="mt-1 text-xs text-text-muted">Custom rules are review-only and can use low or medium risk levels.</p>
        </div>
        <button className="text-xs text-text-muted hover:text-text-primary" onClick={onCancel}>
          Cancel
        </button>
      </div>

      <div className="grid grid-cols-1 gap-3 md:grid-cols-[1fr_1fr_150px]">
        <input
          value={value.name}
          onChange={(e) => setValue({ ...value, name: e.target.value })}
          placeholder="Rule name"
          className="rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60"
        />
        <input
          value={value.pattern}
          onChange={(e) => setValue({ ...value, pattern: e.target.value })}
          placeholder="Glob or text pattern, e.g. */ShaderCache"
          className="rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 font-mono text-xs text-text-primary outline-none focus:border-accent/60"
        />
        <select
          value={value.risk_level}
          onChange={(e) => setValue({ ...value, risk_level: e.target.value as RuleEditorValue["risk_level"] })}
          className="rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60"
        >
          <option value="low">low</option>
          <option value="medium">medium</option>
        </select>
      </div>

      <div className="mt-4">
        <RuleTester pattern={value.pattern} />
      </div>

      <div className="mt-4 flex justify-end">
        <button
          className="rounded-xl border border-accent/30 bg-accent/15 px-5 py-2.5 text-sm font-semibold text-accent-light disabled:cursor-not-allowed disabled:opacity-50"
          onClick={() => void onSave(value)}
          disabled={saving || !canSave}
        >
          {saving ? "Saving..." : "Save Rule"}
        </button>
      </div>
    </div>
  );
}

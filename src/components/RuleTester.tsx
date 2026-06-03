import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface RuleTesterProps {
  pattern: string;
}

export default function RuleTester({ pattern }: RuleTesterProps) {
  const [testPath, setTestPath] = useState("");
  const [result, setResult] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);

  async function handleTest() {
    setTesting(true);
    setError(null);
    setResult(null);
    try {
      setResult(await invoke<boolean>("test_rule_pattern", { pattern, testPath }));
    } catch (e) {
      setError(String(e));
    } finally {
      setTesting(false);
    }
  }

  return (
    <div className="rounded-2xl border border-aurora-border/40 bg-aurora-elevated/40 p-4">
      <div className="mb-3 flex items-center justify-between gap-3">
        <div>
          <div className="text-sm font-semibold text-text-primary">Pattern tester</div>
          <p className="mt-1 text-xs text-text-muted">Try a sample path before saving the rule.</p>
        </div>
        {result !== null && (
          <span className={`rounded-full border px-3 py-1 text-xs font-semibold ${result ? "border-success/25 bg-risk-low-bg text-success" : "border-danger/25 bg-risk-high-bg/30 text-danger"}`}>
            {result ? "Match" : "No match"}
          </span>
        )}
      </div>
      <div className="grid grid-cols-1 gap-3 md:grid-cols-[1fr_auto]">
        <input
          value={testPath}
          onChange={(e) => setTestPath(e.target.value)}
          placeholder="C:\\Users\\alice\\AppData\\Local\\ShaderCache"
          className="rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 font-mono text-xs text-text-primary outline-none focus:border-accent/60"
        />
        <button
          className="rounded-xl border border-accent/30 bg-accent/15 px-4 py-2.5 text-sm font-semibold text-accent-light disabled:cursor-not-allowed disabled:opacity-50"
          onClick={() => void handleTest()}
          disabled={testing || !pattern.trim() || !testPath.trim()}
        >
          {testing ? "Testing..." : "Test"}
        </button>
      </div>
      {error && <p className="mt-2 text-xs text-danger">{error}</p>}
    </div>
  );
}

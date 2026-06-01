import { useState } from "react";

const STEPS = [
  "Select drive",
  "Scan",
  "Review results",
  "Confirm cleanup",
  "Execution summary",
];

export default function CleanupWizard({ selectedDrive, onStartScan }: { selectedDrive: string; onStartScan: (drive: string) => void }) {
  const [step, setStep] = useState(0);

  return (
    <div className="p-8 space-y-6">
      <div>
        <h2 className="text-sm font-semibold uppercase tracking-wider text-text-primary">Cleanup Wizard</h2>
        <p className="mt-1 text-xs text-text-muted">Guided flow that reuses scan, risk classification, cleanup preview, and safe Recycle Bin execution.</p>
      </div>
      <div className="glass-card p-6">
        <div className="grid grid-cols-1 gap-3 md:grid-cols-5">
          {STEPS.map((label, index) => (
            <button key={label} className={`rounded-2xl border p-4 text-left transition-colors ${index === step ? "border-accent/40 bg-accent/15 text-accent-light" : "border-aurora-border/40 bg-aurora-elevated/50 text-text-secondary"}`} onClick={() => setStep(index)}>
              <div className="font-mono text-xs">Step {index + 1}</div>
              <div className="mt-2 text-sm font-semibold">{label}</div>
            </button>
          ))}
        </div>
        <div className="mt-8 rounded-2xl border border-aurora-border/40 bg-aurora-elevated/40 p-6">
          <h3 className="text-lg font-semibold text-text-primary">{STEPS[step]}</h3>
          <p className="mt-2 text-sm leading-6 text-text-secondary">This wizard is the v0.3.8 guided entry point. It keeps cleanup decisions inside the existing preview and safety pipeline.</p>
          {step === 0 && <button className="btn-primary mt-5" onClick={() => onStartScan(selectedDrive)}><span>Scan {selectedDrive}: Drive</span></button>}
        </div>
      </div>
    </div>
  );
}

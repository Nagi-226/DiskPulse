import { useTheme } from "../hooks/useTheme";

export default function ThemeSwitcher() {
  const { resolvedTheme, toggleTheme } = useTheme();

  return (
    <button
      type="button"
      className="rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-3 py-2 text-xs font-semibold text-text-secondary transition-colors hover:border-accent/40 hover:text-accent-light"
      onClick={toggleTheme}
      title="Toggle Aurora theme"
    >
      {resolvedTheme === "dark" ? "Dark" : "Light"}
    </button>
  );
}

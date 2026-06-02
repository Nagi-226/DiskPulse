import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CleanResult, NotificationRecord } from "../types";

const POLL_MS = 30_000;

export default function NotificationCenter() {
  const [open, setOpen] = useState(false);
  const [items, setItems] = useState<NotificationRecord[]>([]);
  const unread = items.filter((item) => !item.read).length;

  async function loadNotifications() {
    setItems(await invoke<NotificationRecord[]>("get_notifications"));
  }

  useEffect(() => {
    void loadNotifications();
    const timer = window.setInterval(() => void loadNotifications(), POLL_MS);
    const unlistenDisk = listen("disk-space-alert", () => void loadNotifications());
    const unlistenCleanup = listen<CleanResult>("cleanup-complete", () => void loadNotifications());
    const unlistenAutoCleanup = listen<CleanResult>("auto-cleanup-complete", () => void loadNotifications());
    const unlistenAutoScheduled = listen("auto-cleanup-scheduled", () => void loadNotifications());

    return () => {
      window.clearInterval(timer);
      unlistenDisk.then((fn) => fn());
      unlistenCleanup.then((fn) => fn());
      unlistenAutoCleanup.then((fn) => fn());
      unlistenAutoScheduled.then((fn) => fn());
    };
  }, []);

  async function markAllRead() {
    await invoke("mark_notifications_read");
    await loadNotifications();
  }

  async function dismiss(id: number) {
    await invoke("mark_notification_read", { id });
    await loadNotifications();
  }

  async function clearAll() {
    await invoke("clear_notifications");
    setItems([]);
  }

  return (
    <div className="relative">
      <button className="relative rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-3 py-2 text-xs font-semibold text-text-secondary hover:text-accent-light" onClick={() => setOpen((value) => !value)}>
        Alerts
        {unread > 0 && <span className="absolute -right-2 -top-2 rounded-full border border-aurora-surface bg-danger px-1.5 py-0.5 text-[10px] text-white">{unread}</span>}
      </button>
      {open && (
        <div className="absolute right-0 top-12 z-20 w-96 max-w-[calc(100vw-2rem)] rounded-2xl border border-aurora-border/60 bg-aurora-surface p-4 shadow-2xl">
          <div className="mb-3 flex items-center justify-between gap-3">
            <div>
              <div className="text-sm font-semibold text-text-primary">Notification Center</div>
              <div className="text-[11px] text-text-muted">Auto-refreshes every 30 seconds.</div>
            </div>
            <div className="flex gap-2">
              <button className="text-xs text-text-muted hover:text-accent-light" onClick={() => void markAllRead()}>Mark read</button>
              <button className="text-xs text-danger hover:text-text-primary" onClick={() => void clearAll()}>Clear all</button>
            </div>
          </div>
          <div className="max-h-96 space-y-2 overflow-y-auto">
            {items.map((item) => (
              <div key={item.id} className={`rounded-xl border p-3 text-xs ${item.read ? "border-aurora-border/40 bg-aurora-elevated/40 text-text-muted" : "border-accent/30 bg-accent/10 text-text-secondary"}`}>
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="truncate font-semibold text-text-primary">{item.title}</div>
                    <div className="mt-1 leading-5">{item.message}</div>
                  </div>
                  {!item.read && <button className="shrink-0 text-[11px] text-text-muted hover:text-accent-light" onClick={() => void dismiss(item.id)}>Dismiss</button>}
                </div>
                <div className="mt-2 font-mono text-[10px] text-text-muted">{item.notification_type} - {item.created_at}</div>
              </div>
            ))}
            {items.length === 0 && <div className="py-8 text-center text-xs text-text-muted">No notifications yet.</div>}
          </div>
        </div>
      )}
    </div>
  );
}

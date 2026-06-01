import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { NotificationRecord } from "../types";

export default function NotificationCenter() {
  const [open, setOpen] = useState(false);
  const [items, setItems] = useState<NotificationRecord[]>([]);
  const unread = items.filter((item) => !item.read).length;

  async function loadNotifications() {
    setItems(await invoke<NotificationRecord[]>("get_notifications"));
  }

  useEffect(() => {
    void loadNotifications();
  }, []);

  async function markAllRead() {
    await invoke("mark_notifications_read");
    await loadNotifications();
  }

  return (
    <div className="relative">
      <button className="rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-3 py-2 text-xs font-semibold text-text-secondary hover:text-accent-light" onClick={() => setOpen((value) => !value)}>
        Alerts {unread > 0 ? `(${unread})` : ""}
      </button>
      {open && (
        <div className="absolute right-0 top-12 z-20 w-80 rounded-2xl border border-aurora-border/60 bg-aurora-surface p-4 shadow-2xl">
          <div className="mb-3 flex items-center justify-between">
            <div className="text-sm font-semibold text-text-primary">Notification Center</div>
            <button className="text-xs text-text-muted hover:text-accent-light" onClick={() => void markAllRead()}>Mark all read</button>
          </div>
          <div className="space-y-2">
            {items.map((item) => (
              <div key={item.id} className="rounded-xl border border-aurora-border/40 bg-aurora-elevated/60 p-3 text-xs text-text-secondary">
                <div className="font-semibold text-text-primary">{item.title}</div>
                <div className="mt-1 leading-5">{item.message}</div>
                <div className="mt-2 font-mono text-[10px] text-text-muted">{item.notification_type} · {item.created_at}</div>
              </div>
            ))}
            {items.length === 0 && <div className="py-8 text-center text-xs text-text-muted">No notifications yet.</div>}
          </div>
        </div>
      )}
    </div>
  );
}

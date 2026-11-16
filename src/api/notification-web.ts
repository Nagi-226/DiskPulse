export async function isPermissionGranted(): Promise<boolean> {
  return typeof Notification !== "undefined" && Notification.permission === "granted";
}

export async function requestPermission(): Promise<NotificationPermission> {
  if (typeof Notification === "undefined") {
    return "denied";
  }
  return Notification.requestPermission();
}

export function sendNotification(options: string | NotificationOptions & { title?: string }) {
  if (typeof Notification === "undefined" || Notification.permission !== "granted") {
    return;
  }

  if (typeof options === "string") {
    new Notification(options);
    return;
  }

  new Notification(options.title ?? "DiskPulse", options);
}

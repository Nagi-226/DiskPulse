export type InvokeArgs = Record<string, unknown> | undefined;

type WebCommandResponse<T> = {
  id: string;
  ok: boolean;
  payload: T;
  error?: string | null;
};

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export function isTauriRuntime() {
  return typeof window !== "undefined" && window.__TAURI_INTERNALS__ != null;
}

async function invokeInTauri<T>(command: string, args?: InvokeArgs): Promise<T> {
  const dynamicImport = new Function("specifier", "return import(specifier)") as (
    specifier: string,
  ) => Promise<typeof import("@tauri-apps/api/core")>;
  const tauri = await dynamicImport("@tauri-apps/api/core");
  return tauri.invoke<T>(command, args);
}

export async function invoke<T = unknown>(command: string, args?: InvokeArgs): Promise<T> {
  if (isTauriRuntime()) {
    return invokeInTauri<T>(command, args);
  }

  const response = await fetch(`/api/cmd/${encodeURIComponent(command)}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(args ?? {}),
  });
  const envelope = (await response.json()) as WebCommandResponse<T>;
  if (!response.ok || !envelope.ok) {
    throw new Error(envelope.error ?? `Web command failed: ${command}`);
  }
  return envelope.payload;
}

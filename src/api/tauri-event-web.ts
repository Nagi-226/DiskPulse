export type UnlistenFn = () => void;

export type Event<T> = {
  event: string;
  id: number;
  payload: T;
};

export type EventCallback<T> = (event: Event<T>) => void;

export async function listen<T>(
  event: string,
  _handler: EventCallback<T>,
): Promise<UnlistenFn> {
  console.debug(`DiskPulse Web Dashboard event bridge is polling-only for '${event}'.`);
  return () => undefined;
}

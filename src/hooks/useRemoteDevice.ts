import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DeviceInfo,
  HubDiscoveryInfo,
  PairingToken,
  CloudDevice,
  RelayStatus,
  RemoteAlertPayload,
  RemoteDeviceRequest,
  RemoteDeviceResponse,
} from "../types";

function toWsUrl(address: string) {
  if (address.startsWith("ws://") || address.startsWith("wss://")) {
    return address;
  }
  return `ws://${address}`;
}

export function useRemoteDevice(defaultPort = 19740) {
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [pairingToken, setPairingToken] = useState<PairingToken | null>(null);
  const [relayStatus, setRelayStatus] = useState<RelayStatus | null>(null);
  const [cloudDevices, setCloudDevices] = useState<CloudDevice[]>([]);
  const [remoteAlerts, setRemoteAlerts] = useState<RemoteAlertPayload[]>([]);
  const [discoveryInfo, setDiscoveryInfo] = useState<HubDiscoveryInfo | null>(null);
  const [isHubRunning, setIsHubRunning] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const unlistenRefs = useRef<UnlistenFn[]>([]);

  const refreshDevices = useCallback(async () => {
    const result = await invoke<DeviceInfo[]>("get_connected_devices");
    setDevices(result);
    return result;
  }, []);

  const refreshCloudDevices = useCallback(async () => {
    const result = await invoke<CloudDevice[]>("list_cloud_devices");
    setCloudDevices(result);
    await refreshDevices();
    return result;
  }, [refreshDevices]);

  const refreshRelayStatus = useCallback(async () => {
    const result = await invoke<RelayStatus>("get_relay_status");
    setRelayStatus(result);
    return result;
  }, []);

  const refreshDiscoveryInfo = useCallback(async () => {
    const result = await invoke<HubDiscoveryInfo | null>("get_hub_discovery_info");
    setDiscoveryInfo(result);
    setIsHubRunning(result != null);
    return result;
  }, []);

  useEffect(() => {
    let mounted = true;
    const listenToHubEvents = async () => {
      const connected = await listen<DeviceInfo>("device-connected", (event) => {
        setDevices((current) => {
          const others = current.filter((device) => device.device_id !== event.payload.device_id);
          return [...others, event.payload].sort((a, b) => a.name.localeCompare(b.name));
        });
      });
      const disconnected = await listen<{ device_id: string }>("device-disconnected", (event) => {
        setDevices((current) =>
          current.map((device) =>
            device.device_id === event.payload.device_id
              ? { ...device, connected: false }
              : device,
          ),
        );
      });
      const alert = await listen<RemoteAlertPayload>("remote-alert", (event) => {
        setRemoteAlerts((current) => [event.payload, ...current].slice(0, 50));
      });

      if (mounted) {
        unlistenRefs.current = [connected, disconnected, alert];
      } else {
        connected();
        disconnected();
        alert();
      }
    };

    void listenToHubEvents();
    void refreshDevices().catch(() => undefined);
    void refreshDiscoveryInfo().catch(() => undefined);
    void refreshRelayStatus().catch(() => undefined);
    void refreshCloudDevices().catch(() => undefined);

    return () => {
      mounted = false;
      for (const unlisten of unlistenRefs.current) {
        unlisten();
      }
      unlistenRefs.current = [];
    };
  }, [refreshCloudDevices, refreshDevices, refreshDiscoveryInfo, refreshRelayStatus]);

  const startHub = useCallback(
    async (port = defaultPort) => {
      setLoading(true);
      setError(null);
      try {
        await invoke("start_hub", { port });
        setIsHubRunning(true);
        await refreshDiscoveryInfo();
        await refreshDevices();
      } catch (e) {
        setError(String(e));
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [defaultPort, refreshDevices, refreshDiscoveryInfo],
  );

  const stopHub = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      await invoke("stop_hub");
      setIsHubRunning(false);
      setDiscoveryInfo(null);
      setDevices([]);
    } catch (e) {
      setError(String(e));
      throw e;
    } finally {
      setLoading(false);
    }
  }, []);

  const connectRelay = useCallback(
    async (url: string) => {
      setLoading(true);
      setError(null);
      try {
        const status = await invoke<RelayStatus>("connect_relay", { url });
        setRelayStatus(status);
        await refreshCloudDevices();
        return status;
      } catch (e) {
        setError(String(e));
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [refreshCloudDevices],
  );

  const disconnectRelay = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const status = await invoke<RelayStatus>("disconnect_relay");
      setRelayStatus(status);
      setCloudDevices([]);
      await refreshDevices();
      return status;
    } catch (e) {
      setError(String(e));
      throw e;
    } finally {
      setLoading(false);
    }
  }, [refreshDevices]);

  const createToken = useCallback(async (deviceName: string, ttlSeconds = 300) => {
    setError(null);
    const token = await invoke<PairingToken>("create_pairing_token", { deviceName, ttlSeconds });
    setPairingToken(token);
    return token;
  }, []);

  const discoverDevices = useCallback(async (timeoutMs = 1500) => {
    setError(null);
    const discovered = await invoke<DeviceInfo[]>("discover_devices", { timeoutMs });
    setDevices((current) => {
      const byId = new Map(current.map((device) => [device.device_id, device]));
      for (const device of discovered) {
        byId.set(device.device_id, { ...byId.get(device.device_id), ...device });
      }
      return Array.from(byId.values()).sort((a, b) => a.name.localeCompare(b.name));
    });
    return discovered;
  }, []);

  const pairDevice = useCallback(async (token: string) => {
    setError(null);
    const device = await invoke<DeviceInfo>("pair_device", { token });
    setDevices((current) => {
      const others = current.filter((item) => item.device_id !== device.device_id);
      return [...others, device].sort((a, b) => a.name.localeCompare(b.name));
    });
    setPairingToken(null);
    return device;
  }, []);

  const unpairDevice = useCallback(async (deviceId: string) => {
    setError(null);
    await invoke("unpair_device", { deviceId });
    setDevices((current) => current.filter((device) => device.device_id !== deviceId));
  }, []);

  const queryRemoteDevice = useCallback(
    async <T,>(device: DeviceInfo, request: RemoteDeviceRequest, timeoutMs = 8000) =>
      new Promise<T>((resolve, reject) => {
        if (device.transport === "cloud") {
          const requestWithId = {
            ...request,
            id: request.id ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`,
          };
          void invoke<RemoteDeviceResponse<T>>("query_cloud_device", {
            deviceId: device.device_id,
            request: requestWithId,
          })
            .then((response) => {
              if (!response.ok) {
                reject(new Error(response.error ?? "Cloud device request failed"));
                return;
              }
              resolve(response.payload);
            })
            .catch(reject);
          return;
        }

        const ws = new WebSocket(toWsUrl(device.address));
        const requestWithId = {
          ...request,
          id: request.id ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`,
        };
        const timeout = window.setTimeout(() => {
          ws.close();
          reject(new Error(`Remote device query timed out after ${timeoutMs}ms`));
        }, timeoutMs);

        ws.onopen = () => {
          ws.send(JSON.stringify(requestWithId));
        };
        ws.onmessage = (event) => {
          window.clearTimeout(timeout);
          ws.close();
          try {
            const response = JSON.parse(String(event.data)) as RemoteDeviceResponse<T>;
            if (!response.ok) {
              reject(new Error(response.error ?? "Remote device request failed"));
              return;
            }
            resolve(response.payload);
          } catch (e) {
            reject(e);
          }
        };
        ws.onerror = () => {
          window.clearTimeout(timeout);
          reject(new Error(`Remote device connection failed: ${device.address}`));
        };
      }),
    [],
  );

  return {
    devices,
    pairingToken,
    relayStatus,
    cloudDevices,
    remoteAlerts,
    discoveryInfo,
    isHubRunning,
    loading,
    error,
    setError,
    refreshDevices,
    refreshCloudDevices,
    refreshRelayStatus,
    refreshDiscoveryInfo,
    startHub,
    stopHub,
    connectRelay,
    disconnectRelay,
    discoverDevices,
    createToken,
    pairDevice,
    unpairDevice,
    queryRemoteDevice,
  };
}

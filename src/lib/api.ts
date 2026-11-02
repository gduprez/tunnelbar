import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  ConfigInfo,
  ConfigProblem,
  LoginItem,
  SettingsView,
  Tunnel,
  TunnelStatus,
  TunnelView,
  ValidationError,
} from './types';

// Every call to the Rust side goes through here, so that components can be tested with a mock.

export const api = {
  listTunnels: () => invoke<TunnelView[]>('list_tunnels'),
  startTunnel: (id: string) => invoke<void>('start_tunnel', { id }),
  stopTunnel: (id: string) => invoke<void>('stop_tunnel', { id }),
  shellCommand: (id: string) => invoke<string>('shell_command', { id }),
  configInfo: () => invoke<ConfigInfo>('get_config_info'),
  validateTunnel: (tunnel: Tunnel, previousId: string | null) =>
    invoke<ValidationError[]>('validate_tunnel', { tunnel, previousId }),
  /** Rejects with a ConfigProblem. */
  saveTunnel: (tunnel: Tunnel, previousId: string | null) =>
    invoke<void>('save_tunnel', { tunnel, previousId }),
  /** Rejects with a ConfigProblem. */
  deleteTunnel: (id: string) => invoke<void>('delete_tunnel', { id }),
  openConfig: () => invoke<void>('open_config'),
  settings: () => invoke<SettingsView>('get_settings'),
  setLanguage: (language: string) => invoke<SettingsView>('set_language', { language }),
  onStatus: (callback: (status: TunnelStatus) => void): Promise<UnlistenFn> =>
    listen<TunnelStatus>('tunnel-status', (event) => callback(event.payload)),
  onTunnelsChanged: (callback: () => void): Promise<UnlistenFn> =>
    listen('tunnels-changed', () => callback()),
  copy: (text: string) => navigator.clipboard.writeText(text),
  /** Resizes the panel to its content; the Rust side bounds the height. */
  fitPanel: (height: number) => invoke<void>('fit_panel', { height }),
  loginItem: () => invoke<LoginItem>('get_login_item'),
  setLoginItem: (enabled: boolean) => invoke<LoginItem>('set_login_item', { enabled }),
};

export type Api = typeof api;
export type { ConfigProblem };

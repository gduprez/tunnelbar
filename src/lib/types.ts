// Mirrors of the JSON sent by the Rust side (see crates/tunnelbar-core/tests/json_shapes.rs).

export type Forward =
  | { type: 'local'; local_port: number; remote_host: string; remote_port: number }
  | { type: 'remote'; remote_port: number; local_host: string; local_port: number }
  | { type: 'dynamic'; local_port: number };

/** A tunnel definition, with the field names of tunnels.toml. */
export interface Tunnel {
  id: string;
  name: string;
  host: string;
  user?: string;
  port?: number;
  autostart: boolean;
  auto_reconnect: boolean;
  accept_new_host_key: boolean;
  forward: Forward[];
}

export interface TunnelError {
  code: string;
  port?: number | null;
  exit_code?: number | null;
}

export type TunnelState =
  | { state: 'stopped' }
  | { state: 'connecting'; attempt: number }
  | { state: 'up' }
  | { state: 'reconnecting'; attempt: number; delay_ms: number; last_error: TunnelError }
  | { state: 'failed'; error: TunnelError };

export interface TunnelStatus {
  id: string;
  state: TunnelState;
  up_since_ms: number | null;
  reconnections: number;
  logs: string[];
}

export interface TunnelView extends Tunnel {
  status: TunnelStatus;
}

export interface ValidationError {
  tunnel: number | null;
  forward: number | null;
  code: string;
  field?: string;
  port?: number;
  id?: string;
  found?: number;
}

export type ConfigProblem =
  | { kind: 'io'; message: string }
  | { kind: 'parse'; message: string; line: number | null }
  | { kind: 'invalid'; errors: ValidationError[] }
  | { kind: 'file_broken' };

export interface ConfigInfo {
  path: string;
  problem: ConfigProblem | null;
}

export interface SettingsView {
  language: string;
  locale: string;
  available: string[];
}

/** Launch at login, as reported by SMAppService. */
export type LoginItem = 'enabled' | 'disabled' | 'requires_approval' | 'unavailable';

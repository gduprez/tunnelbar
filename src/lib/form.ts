import type { Forward, Tunnel, ValidationError } from './types';

// Form fields hold text as typed; conversion to a Tunnel happens on validation and save.

export type ForwardForm =
  | { type: 'local'; local_port: string; remote_host: string; remote_port: string }
  | { type: 'remote'; remote_port: string; local_host: string; local_port: string }
  | { type: 'dynamic'; local_port: string };

export interface TunnelForm {
  id: string;
  name: string;
  host: string;
  user: string;
  port: string;
  autostart: boolean;
  auto_reconnect: boolean;
  accept_new_host_key: boolean;
  forwards: ForwardForm[];
}

/** An id suggestion from a name: `PostgreSQL prod` → `postgresql-prod`. */
export function slugify(name: string): string {
  return name
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+/, '')
    .slice(0, 64)
    .replace(/-+$/, '');
}

export function emptyForm(): TunnelForm {
  return {
    id: '',
    name: '',
    host: '',
    user: '',
    port: '',
    autostart: false,
    auto_reconnect: true,
    accept_new_host_key: false,
    forwards: [{ type: 'local', local_port: '', remote_host: '127.0.0.1', remote_port: '' }],
  };
}

export function fromTunnel(tunnel: Tunnel): TunnelForm {
  return {
    id: tunnel.id,
    name: tunnel.name,
    host: tunnel.host,
    user: tunnel.user ?? '',
    port: tunnel.port?.toString() ?? '',
    autostart: tunnel.autostart,
    auto_reconnect: tunnel.auto_reconnect,
    accept_new_host_key: tunnel.accept_new_host_key,
    forwards: tunnel.forward.map(forwardForm),
  };
}

function forwardForm(forward: Forward): ForwardForm {
  switch (forward.type) {
    case 'local':
      return {
        type: 'local',
        local_port: String(forward.local_port),
        remote_host: forward.remote_host,
        remote_port: String(forward.remote_port),
      };
    case 'remote':
      return {
        type: 'remote',
        remote_port: String(forward.remote_port),
        local_host: forward.local_host,
        local_port: String(forward.local_port),
      };
    case 'dynamic':
      return { type: 'dynamic', local_port: String(forward.local_port) };
  }
}

/** A valid port number, or 0 (which validation reports) for anything else. */
function port(text: string): number {
  const trimmed = text.trim();
  const value = /^\d+$/.test(trimmed) ? Number(trimmed) : 0;
  return value >= 1 && value <= 65535 ? value : 0;
}

export function toTunnel(form: TunnelForm): Tunnel {
  const tunnel: Tunnel = {
    id: form.id.trim(),
    name: form.name.trim(),
    host: form.host.trim(),
    autostart: form.autostart,
    auto_reconnect: form.auto_reconnect,
    accept_new_host_key: form.accept_new_host_key,
    forward: form.forwards.map(toForward),
  };
  if (form.user.trim()) tunnel.user = form.user.trim();
  if (form.port.trim()) tunnel.port = port(form.port);
  return tunnel;
}

function toForward(forward: ForwardForm): Forward {
  switch (forward.type) {
    case 'local':
      return {
        type: 'local',
        local_port: port(forward.local_port),
        remote_host: forward.remote_host.trim(),
        remote_port: port(forward.remote_port),
      };
    case 'remote':
      return {
        type: 'remote',
        remote_port: port(forward.remote_port),
        local_host: forward.local_host.trim(),
        local_port: port(forward.local_port),
      };
    case 'dynamic':
      return { type: 'dynamic', local_port: port(forward.local_port) };
  }
}

const TUNNEL_FIELDS: Record<string, string> = {
  unsupported_version: 'form',
  invalid_id: 'id',
  duplicate_id: 'id',
  empty_name: 'name',
  invalid_host: 'host',
  invalid_user: 'user',
  no_forward: 'forwards',
};

/** Validation errors grouped by form field (`host`, `forward.0.local_port`, …). */
export function fieldErrors(errors: ValidationError[]): Record<string, ValidationError[]> {
  const grouped: Record<string, ValidationError[]> = {};
  for (const error of errors) {
    const field = fieldOf(error);
    (grouped[field] ??= []).push(error);
  }
  return grouped;
}

function fieldOf(error: ValidationError): string {
  if (error.forward == null) {
    return error.code === 'zero_port' ? 'port' : (TUNNEL_FIELDS[error.code] ?? 'form');
  }
  const prefix = `forward.${error.forward}`;
  if (error.code === 'invalid_forward_host') return `${prefix}.host`;
  if (error.code === 'local_port_conflict') return `${prefix}.local_port`;
  return `${prefix}.${error.field ?? 'local_port'}`;
}

// Development only: `npm run dev`, then open http://localhost:1420/?preview to work on the
// panel in a browser, with sample tunnels instead of the Rust side.

import { mockIPC } from '@tauri-apps/api/mocks';
import type { TunnelView } from './types';

const now = Date.now();

const tunnels: TunnelView[] = [
  {
    id: 'prod-db',
    name: 'PostgreSQL prod',
    host: '203.0.113.10',
    autostart: false,
    auto_reconnect: true,
    accept_new_host_key: false,
    forward: [{ type: 'local', local_port: 54321, remote_host: '127.0.0.1', remote_port: 5432 }],
    status: {
      id: 'prod-db',
      state: { state: 'up' },
      up_since_ms: now - 3_725_000,
      reconnections: 1,
      logs: ['Timeout, server 203.0.113.10 not responding.'],
    },
  },
  {
    id: 'staging',
    name: 'Staging (DB + Redis + SOCKS)',
    host: 'staging-bastion',
    user: 'deploy',
    port: 2222,
    autostart: true,
    auto_reconnect: true,
    accept_new_host_key: false,
    forward: [
      { type: 'local', local_port: 54322, remote_host: 'db.internal', remote_port: 5432 },
      { type: 'dynamic', local_port: 1080 },
    ],
    status: {
      id: 'staging',
      state: {
        state: 'reconnecting',
        attempt: 3,
        delay_ms: 4000,
        last_error: { code: 'connection_refused' },
      },
      up_since_ms: null,
      reconnections: 0,
      logs: ['ssh: connect to host staging-bastion port 2222: Connection refused'],
    },
  },
  {
    id: 'client-x',
    name: 'Client X bastion',
    host: 'bastion.client-x.example',
    autostart: false,
    auto_reconnect: true,
    accept_new_host_key: false,
    forward: [{ type: 'local', local_port: 33060, remote_host: '10.0.0.5', remote_port: 3306 }],
    status: {
      id: 'client-x',
      state: { state: 'failed', error: { code: 'auth_failed' } },
      up_since_ms: null,
      reconnections: 0,
      logs: ['deploy@bastion.client-x.example: Permission denied (publickey).'],
    },
  },
  {
    id: 'redis',
    name: 'Redis cache',
    host: 'cache.example.com',
    autostart: false,
    auto_reconnect: true,
    accept_new_host_key: false,
    forward: [{ type: 'local', local_port: 63790, remote_host: '127.0.0.1', remote_port: 6379 }],
    status: {
      id: 'redis',
      state: { state: 'stopped' },
      up_since_ms: null,
      reconnections: 0,
      logs: [],
    },
  },
];

export function installPreview() {
  let language = 'auto';
  mockIPC((command, args) => {
    const params = args as Record<string, unknown>;
    switch (command) {
      case 'list_tunnels':
        return tunnels;
      case 'get_config_info':
        return { path: '~/Library/Application Support/TunnelBar/tunnels.toml', problem: null };
      case 'get_settings':
      case 'set_language':
        if (command === 'set_language') language = params.language as string;
        return { language, locale: language === 'auto' ? 'en' : language, available: ['en', 'fr'] };
      case 'get_platform':
        return { os: 'macos', tray_available: true };
      case 'fit_panel':
        return null;
      case 'get_login_item':
        return 'disabled';
      case 'set_login_item':
        return params.enabled ? 'enabled' : 'disabled';
      case 'validate_tunnel':
        return [];
      case 'shell_command':
        return '/usr/bin/ssh -N -L 54321:127.0.0.1:5432 -- 203.0.113.10';
      default:
        return null;
    }
  });
}

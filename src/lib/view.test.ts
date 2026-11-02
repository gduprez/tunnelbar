import { describe, expect, it } from 'vitest';
import { createTranslator } from './i18n';
import { catalogs } from './locales';
import type { TunnelState, TunnelView } from './types';
import {
  errorTitle,
  listSummary,
  localAddress,
  localPorts,
  rowSubtitle,
  stateTone,
  suggestPort,
} from './view';

const t = createTranslator(catalogs, 'en');
const NOW = 1_800_000_000_000;

function view(state: TunnelState, overrides: Partial<TunnelView> = {}): TunnelView {
  return {
    id: 'prod',
    name: 'Prod',
    host: 'db.example.com',
    autostart: false,
    auto_reconnect: true,
    accept_new_host_key: false,
    forward: [{ type: 'local', local_port: 54321, remote_host: '127.0.0.1', remote_port: 5432 }],
    status: { id: 'prod', state, up_since_ms: null, reconnections: 0, logs: [] },
    ...overrides,
  };
}

const failed: TunnelState = { state: 'failed', error: { code: 'auth_failed' } };
const retrying: TunnelState = {
  state: 'reconnecting',
  attempt: 3,
  delay_ms: 4000,
  last_error: { code: 'connection_lost' },
};

describe('stateTone', () => {
  it('gives each state its colour family', () => {
    expect(stateTone({ state: 'up' })).toBe('up');
    expect(stateTone({ state: 'connecting', attempt: 0 })).toBe('pending');
    expect(stateTone(retrying)).toBe('retry');
    expect(stateTone(failed)).toBe('error');
    expect(stateTone({ state: 'stopped' })).toBe('off');
  });
});

describe('localAddress', () => {
  it('shows the first local port, and how many other forwards there are', () => {
    expect(localAddress(view({ state: 'up' }))).toBe('localhost:54321');
    const several = view(
      { state: 'up' },
      {
        forward: [
          { type: 'local', local_port: 54321, remote_host: 'x', remote_port: 1 },
          { type: 'dynamic', local_port: 1080 },
        ],
      },
    );
    expect(localAddress(several)).toBe('localhost:54321 +1');
  });

  it('falls back to the remote port of remote-only tunnels', () => {
    const remote = view(
      { state: 'up' },
      {
        forward: [{ type: 'remote', remote_port: 8080, local_host: 'localhost', local_port: 3000 }],
      },
    );
    expect(localAddress(remote)).toBe('remote:8080');
  });
});

describe('rowSubtitle', () => {
  it('shows the address with the uptime or the stopped state', () => {
    const up = view({ state: 'up' });
    up.status.up_since_ms = NOW - 2 * 3_600_000;
    expect(rowSubtitle(t, up, NOW)).toBe('localhost:54321 · active for 2 h 00 min');
    expect(rowSubtitle(t, view({ state: 'stopped' }), NOW)).toBe('localhost:54321 · stopped');
  });

  it('explains transient and failed states in a few words', () => {
    expect(rowSubtitle(t, view({ state: 'connecting', attempt: 0 }), NOW)).toBe('Connecting…');
    expect(rowSubtitle(t, view(retrying), NOW)).toBe('Connection lost · retrying in 4 s');
    expect(rowSubtitle(t, view(failed), NOW)).toBe('Authentication refused');
  });
});

describe('errorTitle', () => {
  it('has a short title for every error code of the core', () => {
    const codes = [
      'auth_failed',
      'host_key_unknown',
      'host_key_mismatch',
      'port_in_use',
      'remote_forward_failed',
      'host_not_found',
      'connection_refused',
      'connection_timed_out',
      'network_unreachable',
      'connection_lost',
      'spawn_failed',
      'unknown',
    ];
    for (const code of codes) {
      expect(errorTitle(t, { code })).not.toBe(`error_title.${code}`);
    }
  });
});

describe('listSummary', () => {
  it('counts running tunnels', () => {
    expect(listSummary(t, [view({ state: 'up' }), view({ state: 'stopped' })])).toEqual({
      text: '1 of 2 running',
      alert: false,
    });
  });

  it('puts tunnels in error first', () => {
    expect(listSummary(t, [view(failed), view({ state: 'up' })])).toEqual({
      text: 'Needs attention: 1',
      alert: true,
    });
  });

  it('says nothing without tunnels', () => {
    expect(listSummary(t, [])).toEqual({ text: '', alert: false });
  });
});

describe('free port suggestion', () => {
  it('lists the local ports used by other tunnels', () => {
    const tunnels = [
      view({ state: 'up' }),
      view({ state: 'up' }, { id: 'other', forward: [{ type: 'dynamic', local_port: 1080 }] }),
    ];
    expect(localPorts(tunnels, 'other')).toEqual([54321]);
    expect(localPorts(tunnels, null)).toEqual([54321, 1080]);
  });

  it('suggests the next free port', () => {
    expect(suggestPort([54321, 54322], 54321)).toBe(54323);
    expect(suggestPort([65535], 65535)).toBeNull();
  });
});

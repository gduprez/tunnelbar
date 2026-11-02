import { describe, expect, it } from 'vitest';
import {
  applyStatus,
  errorMessage,
  formatDuration,
  forwardSummary,
  isRunning,
  stateText,
  tunnelTarget,
  validationMessage,
} from './format';
import { createTranslator } from './i18n';
import { catalogs } from './locales';
import type { Tunnel, TunnelStatus, TunnelView } from './types';

const t = createTranslator(catalogs, 'en');

const tunnel: Tunnel = {
  id: 'prod',
  name: 'Prod',
  host: '203.0.113.10',
  autostart: false,
  auto_reconnect: true,
  accept_new_host_key: false,
  forward: [{ type: 'local', local_port: 54321, remote_host: '127.0.0.1', remote_port: 5432 }],
};

describe('forwardSummary', () => {
  it('describes each kind of forward', () => {
    expect(forwardSummary(tunnel.forward[0]!)).toBe('localhost:54321 → 127.0.0.1:5432');
    expect(
      forwardSummary({
        type: 'remote',
        remote_port: 8080,
        local_host: '127.0.0.1',
        local_port: 3000,
      }),
    ).toBe('remote:8080 → 127.0.0.1:3000');
    expect(forwardSummary({ type: 'dynamic', local_port: 1080 })).toBe('SOCKS localhost:1080');
  });
});

describe('tunnelTarget', () => {
  it('shows user and port only when set', () => {
    expect(tunnelTarget(tunnel)).toBe('203.0.113.10');
    expect(tunnelTarget({ ...tunnel, user: 'deploy', port: 2222 })).toBe(
      'deploy@203.0.113.10:2222',
    );
  });
});

describe('isRunning', () => {
  it('is true while connecting, up or reconnecting', () => {
    expect(isRunning({ state: 'up' })).toBe(true);
    expect(isRunning({ state: 'connecting', attempt: 0 })).toBe(true);
    expect(
      isRunning({ state: 'reconnecting', attempt: 1, delay_ms: 1000, last_error: { code: 'x' } }),
    ).toBe(true);
    expect(isRunning({ state: 'stopped' })).toBe(false);
    expect(isRunning({ state: 'failed', error: { code: 'auth_failed' } })).toBe(false);
  });
});

describe('stateText', () => {
  it('names each state, with the retry delay when reconnecting', () => {
    expect(stateText(t, { state: 'up' })).toBe('Active');
    expect(stateText(t, { state: 'stopped' })).toBe('Stopped');
    expect(stateText(t, { state: 'connecting', attempt: 0 })).toBe('Connecting…');
    expect(
      stateText(t, {
        state: 'reconnecting',
        attempt: 2,
        delay_ms: 4000,
        last_error: { code: 'connection_lost' },
      }),
    ).toBe('Reconnecting in 4 s (attempt 2)');
    expect(stateText(t, { state: 'failed', error: { code: 'auth_failed' } })).toBe('Error');
  });
});

describe('errorMessage', () => {
  it('translates every error code of the core', () => {
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
      expect(errorMessage(t, { code })).not.toBe(`error.${code}`);
    }
  });

  it('includes the port when known', () => {
    expect(errorMessage(t, { code: 'port_in_use', port: 54321 })).toContain('54321');
    expect(errorMessage(t, { code: 'port_in_use', port: null })).not.toContain('{port}');
  });
});

describe('validationMessage', () => {
  it('translates every validation code of the core', () => {
    const codes = [
      'unsupported_version',
      'invalid_id',
      'duplicate_id',
      'empty_name',
      'invalid_host',
      'invalid_user',
      'invalid_forward_host',
      'zero_port',
      'no_forward',
      'local_port_conflict',
    ];
    for (const code of codes) {
      expect(validationMessage(t, { tunnel: 0, forward: null, code })).not.toBe(
        `validation.${code}`,
      );
    }
    expect(
      validationMessage(t, { tunnel: 0, forward: 0, code: 'local_port_conflict', port: 5432 }),
    ).toContain('5432');
  });
});

describe('formatDuration', () => {
  it('uses the two most significant units', () => {
    expect(formatDuration(42_000)).toBe('42 s');
    expect(formatDuration(5 * 60_000 + 3_000)).toBe('5 min 03 s');
    expect(formatDuration(2 * 3_600_000 + 5 * 60_000)).toBe('2 h 05 min');
    expect(formatDuration(3 * 86_400_000 + 3_600_000)).toBe('3 d 1 h');
    expect(formatDuration(-5)).toBe('0 s');
  });
});

describe('applyStatus', () => {
  it('replaces the status of the matching tunnel only', () => {
    const stopped: TunnelStatus = {
      id: 'prod',
      state: { state: 'stopped' },
      up_since_ms: null,
      reconnections: 0,
      logs: [],
    };
    const views: TunnelView[] = [
      { ...tunnel, status: stopped },
      { ...tunnel, id: 'other', status: { ...stopped, id: 'other' } },
    ];
    const up: TunnelStatus = { ...stopped, state: { state: 'up' } };
    const next = applyStatus(views, up);
    expect(next[0]!.status).toEqual(up);
    expect(next[1]).toBe(views[1]);
    expect(applyStatus(views, { ...up, id: 'unknown' })).toEqual(views);
  });
});

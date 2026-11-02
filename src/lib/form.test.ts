import { describe, expect, it } from 'vitest';
import { emptyForm, fieldErrors, fromTunnel, slugify, toTunnel, type TunnelForm } from './form';
import type { Tunnel } from './types';

const tunnel: Tunnel = {
  id: 'prod',
  name: 'Prod',
  host: '203.0.113.10',
  autostart: false,
  auto_reconnect: true,
  accept_new_host_key: false,
  forward: [
    { type: 'local', local_port: 54321, remote_host: '127.0.0.1', remote_port: 5432 },
    { type: 'dynamic', local_port: 1080 },
  ],
};

describe('slugify', () => {
  it('turns a name into a valid id', () => {
    expect(slugify('PostgreSQL prod')).toBe('postgresql-prod');
    expect(slugify('  Base de données (été) ')).toBe('base-de-donnees-ete');
    expect(slugify('---')).toBe('');
    expect(slugify('x'.repeat(80))).toHaveLength(64);
  });
});

describe('form conversion', () => {
  it('round-trips a tunnel, including forwards the form does not edit', () => {
    expect(toTunnel(fromTunnel(tunnel))).toEqual(tunnel);
  });

  it('keeps optional fields absent when blank', () => {
    const form: TunnelForm = { ...fromTunnel(tunnel), user: '  ', port: '' };
    const result = toTunnel(form);
    expect(result).not.toHaveProperty('user');
    expect(result).not.toHaveProperty('port');
  });

  it('trims text and parses numbers', () => {
    const form = fromTunnel(tunnel);
    form.host = ' db.example.com ';
    form.user = ' deploy ';
    form.port = '2222';
    const result = toTunnel(form);
    expect(result.host).toBe('db.example.com');
    expect(result.user).toBe('deploy');
    expect(result.port).toBe(2222);
  });

  it('maps invalid or out-of-range ports to 0, which validation rejects', () => {
    const form = fromTunnel(tunnel);
    form.port = '70000';
    form.forwards[0] = { type: 'local', local_port: 'abc', remote_host: 'x', remote_port: '-1' };
    const result = toTunnel(form);
    expect(result.port).toBe(0);
    expect(result.forward[0]).toEqual({
      type: 'local',
      local_port: 0,
      remote_host: 'x',
      remote_port: 0,
    });
  });

  it('starts new tunnels with one local forward and reconnection on', () => {
    const form = emptyForm();
    expect(form.auto_reconnect).toBe(true);
    expect(form.forwards).toEqual([
      { type: 'local', local_port: '', remote_host: '127.0.0.1', remote_port: '' },
    ]);
  });
});

describe('fieldErrors', () => {
  it('attaches validation errors to form fields', () => {
    const errors = fieldErrors([
      { tunnel: 0, forward: null, code: 'invalid_host' },
      { tunnel: 0, forward: null, code: 'zero_port', field: 'ssh_port' },
      { tunnel: 0, forward: 1, code: 'zero_port', field: 'remote_port' },
      { tunnel: 0, forward: 1, code: 'local_port_conflict', port: 5 },
      { tunnel: 0, forward: 0, code: 'invalid_forward_host' },
      { tunnel: 0, forward: null, code: 'no_forward' },
    ]);
    expect(Object.keys(errors).sort()).toEqual(
      [
        'forward.0.host',
        'forward.1.local_port',
        'forward.1.remote_port',
        'forwards',
        'host',
        'port',
      ].sort(),
    );
    expect(errors['host']?.[0]?.code).toBe('invalid_host');
  });
});

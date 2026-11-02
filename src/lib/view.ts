import { formatDuration, type Translate } from './format';
import type { Tunnel, TunnelError, TunnelState, TunnelView } from './types';

// What the panel shows of a tunnel, beyond the raw state: colours, one-line summaries.

export type Tone = 'up' | 'pending' | 'retry' | 'error' | 'off';

export function stateTone(state: TunnelState): Tone {
  switch (state.state) {
    case 'up':
      return 'up';
    case 'connecting':
      return 'pending';
    case 'reconnecting':
      return 'retry';
    case 'failed':
      return 'error';
    case 'stopped':
      return 'off';
  }
}

/** The address to use on this Mac: `localhost:54321`, `+N` for the other forwards. */
export function localAddress(tunnel: Tunnel): string {
  const [first, ...others] = tunnel.forward;
  if (!first) return '';
  const address =
    first.type === 'remote' ? `remote:${first.remote_port}` : `localhost:${first.local_port}`;
  return others.length > 0 ? `${address} +${others.length}` : address;
}

export function errorTitle(t: Translate, error: TunnelError): string {
  return t(`error_title.${error.code}`);
}

/** The second line of a tunnel in the list. */
export function rowSubtitle(t: Translate, tunnel: TunnelView, now: number): string {
  const state = tunnel.status.state;
  const address = localAddress(tunnel);
  switch (state.state) {
    case 'up':
      return tunnel.status.up_since_ms == null
        ? address
        : `${address} · ${t('row.up_since', { duration: formatDuration(now - tunnel.status.up_since_ms) })}`;
    case 'stopped':
      return `${address} · ${t('row.stopped')}`;
    case 'connecting':
      return t('state.connecting');
    case 'reconnecting':
      return `${errorTitle(t, state.last_error)} · ${t('row.retry_in', {
        seconds: Math.round(state.delay_ms / 1000),
      })}`;
    case 'failed':
      return errorTitle(t, state.error);
  }
}

/** The header line: what needs attention first, else how many tunnels run. */
export function listSummary(t: Translate, tunnels: TunnelView[]): { text: string; alert: boolean } {
  if (tunnels.length === 0) return { text: '', alert: false };
  const failed = tunnels.filter((tunnel) => tunnel.status.state.state === 'failed').length;
  if (failed > 0) return { text: t('list.attention', { count: failed }), alert: true };
  const running = tunnels.filter((tunnel) => stateTone(tunnel.status.state) !== 'off').length;
  return { text: t('list.running', { running, total: tunnels.length }), alert: false };
}

/** Ports listened on by the forwards of every tunnel but `excludeId`. */
export function localPorts(tunnels: Tunnel[], excludeId: string | null): number[] {
  return tunnels
    .filter((tunnel) => tunnel.id !== excludeId)
    .flatMap((tunnel) => tunnel.forward)
    .flatMap((forward) => (forward.type === 'remote' ? [] : [forward.local_port]));
}

/** The first port after `from` that no other tunnel uses. */
export function suggestPort(used: number[], from: number): number | null {
  for (let port = from + 1; port <= 65535; port++) {
    if (!used.includes(port)) return port;
  }
  return null;
}

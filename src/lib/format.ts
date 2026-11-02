import type {
  Forward,
  Tunnel,
  TunnelError,
  TunnelState,
  TunnelStatus,
  TunnelView,
  ValidationError,
} from './types';

export type Translate = (key: string, params?: Record<string, string | number>) => string;

export function forwardSummary(forward: Forward): string {
  switch (forward.type) {
    case 'local':
      return `localhost:${forward.local_port} → ${forward.remote_host}:${forward.remote_port}`;
    case 'remote':
      return `remote:${forward.remote_port} → ${forward.local_host}:${forward.local_port}`;
    case 'dynamic':
      return `SOCKS localhost:${forward.local_port}`;
  }
}

/** `user@host:port`, with only the parts that are set. */
export function tunnelTarget(tunnel: Tunnel): string {
  const user = tunnel.user ? `${tunnel.user}@` : '';
  const port = tunnel.port ? `:${tunnel.port}` : '';
  return `${user}${tunnel.host}${port}`;
}

/** Running tunnels are switched on: a toggle stops them. */
export function isRunning(state: TunnelState): boolean {
  return state.state === 'connecting' || state.state === 'up' || state.state === 'reconnecting';
}

export function stateText(t: Translate, state: TunnelState): string {
  if (state.state === 'reconnecting') {
    return t('state.reconnecting', {
      seconds: Math.round(state.delay_ms / 1000),
      attempt: state.attempt,
    });
  }
  return t(`state.${state.state}`);
}

export function errorMessage(t: Translate, error: TunnelError): string {
  const key = error.port == null ? `error.${error.code}` : `error.${error.code}.port`;
  return t(key, { port: error.port ?? '', exit_code: error.exit_code ?? '?' });
}

export function validationMessage(t: Translate, error: ValidationError): string {
  return t(`validation.${error.code}`, {
    port: error.port ?? '',
    id: error.id ?? '',
    found: error.found ?? '',
  });
}

/** Two most significant units: `42 s`, `5 min 03 s`, `2 h 05 min`, `3 d 1 h`. */
export function formatDuration(ms: number): string {
  const seconds = Math.max(0, Math.floor(ms / 1000));
  const pad = (n: number) => String(n).padStart(2, '0');
  const [d, h, m, s] = [
    Math.floor(seconds / 86_400),
    Math.floor(seconds / 3_600) % 24,
    Math.floor(seconds / 60) % 60,
    seconds % 60,
  ];
  if (d > 0) return `${d} d ${h} h`;
  if (h > 0) return `${h} h ${pad(m)} min`;
  if (m > 0) return `${m} min ${pad(s)} s`;
  return `${s} s`;
}

export function applyStatus(views: TunnelView[], status: TunnelStatus): TunnelView[] {
  return views.map((view) => (view.id === status.id ? { ...view, status } : view));
}

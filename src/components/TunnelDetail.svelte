<script lang="ts">
  import {
    errorMessage,
    formatDuration,
    isRunning,
    stateText,
    tunnelTarget,
    type Translate,
  } from '../lib/format';
  import type { Forward, TunnelView } from '../lib/types';
  import { errorTitle, stateTone } from '../lib/view';
  import StatusDot from './StatusDot.svelte';

  let {
    tunnel,
    t,
    now,
    confirmingDelete,
    ontoggle,
    ondelete,
    oncanceldelete,
    oncopylogs,
  }: {
    tunnel: TunnelView;
    t: Translate;
    now: number;
    confirmingDelete: boolean;
    ontoggle: () => void;
    ondelete: () => void;
    oncanceldelete: () => void;
    oncopylogs: () => void;
  } = $props();

  const current = $derived(tunnel.status.state);
  const tone = $derived(stateTone(current));
  const running = $derived(isRunning(current));

  /** Second line of the status card. */
  const detail = $derived.by(() => {
    switch (current.state) {
      case 'up': {
        const since =
          tunnel.status.up_since_ms == null
            ? ''
            : t('detail.uptime', { duration: formatDuration(now - tunnel.status.up_since_ms) });
        const reconnections =
          tunnel.status.reconnections > 0
            ? t('detail.reconnections', { count: tunnel.status.reconnections })
            : t('detail.no_reconnection');
        return [since, reconnections].filter(Boolean).join(' · ');
      }
      case 'reconnecting':
        return errorTitle(t, current.last_error);
      default:
        return '';
    }
  });

  function rows(forward: Forward): [string, string][] {
    switch (forward.type) {
      case 'local':
        return [
          [t('detail.on_this_mac'), `localhost:${forward.local_port}`],
          [t('detail.to'), `${forward.remote_host}:${forward.remote_port}`],
        ];
      case 'remote':
        return [
          [t('detail.on_server'), `port ${forward.remote_port}`],
          [t('detail.to'), `${forward.local_host}:${forward.local_port}`],
        ];
      case 'dynamic':
        return [[t('detail.socks'), `localhost:${forward.local_port}`]];
    }
  }
</script>

<section class="detail">
  <div class="status {tone}">
    <div class="headline">
      <StatusDot {tone} large />
      <div class="words">
        <strong>{stateText(t, current)}</strong>
        {#if detail}<span class="secondary">{detail}</span>{/if}
      </div>
    </div>
    {#if current.state === 'failed'}
      <p class="explanation" role="alert">{errorMessage(t, current.error)}</p>
    {/if}
    <button type="button" class:primary={!running} onclick={ontoggle}>
      {running ? t('actions.stop_tunnel') : t('actions.start_tunnel')}
    </button>
  </div>

  <h2 class="section-title">{t('detail.access')}</h2>
  {#each tunnel.forward as forward, index (index)}
    <dl class="group">
      {#each rows(forward) as [label, value] (label)}
        <div class="group-row">
          <dt>{label}</dt>
          <dd class="mono">{value}</dd>
        </div>
      {/each}
    </dl>
  {/each}
  <dl class="group via">
    <div class="group-row">
      <dt>{t('detail.via_label')}</dt>
      <dd class="mono">{tunnelTarget(tunnel)}</dd>
    </div>
  </dl>

  <h2 class="section-title">{t('detail.logs')}</h2>
  <details class="group logs" open={current.state === 'failed' || current.state === 'reconnecting'}>
    <summary class="group-row">
      <span>
        {tunnel.status.logs.length === 0
          ? t('detail.nothing_reported')
          : t('detail.lines', { count: tunnel.status.logs.length })}
      </span>
      <svg
        width="12"
        height="12"
        viewBox="0 0 12 12"
        fill="none"
        stroke="currentColor"
        stroke-width="1.6"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"><path d="m4.5 2.5 3.5 3.5-3.5 3.5" /></svg
      >
    </summary>
    {#if tunnel.status.logs.length > 0}
      <pre class="mono">{tunnel.status.logs.join('\n')}</pre>
      <button type="button" class="link copy" onclick={oncopylogs}>{t('actions.copy_logs')}</button>
    {/if}
  </details>

  {#if confirmingDelete}
    <div class="confirm" role="alertdialog" aria-labelledby="delete-question">
      <p id="delete-question">{t('detail.delete_confirm', { name: tunnel.name })}</p>
      <div class="buttons">
        <button type="button" onclick={oncanceldelete}>{t('common.cancel')}</button>
        <button type="button" class="danger" onclick={ondelete}>{t('common.delete')}</button>
      </div>
    </div>
  {/if}
</section>

<style>
  .detail {
    padding-bottom: 14px;
  }

  .status {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 4px 12px 0;
    padding: 14px;
    border: 1px solid var(--separator);
    border-radius: 12px;
    background: var(--surface);
  }

  .status.up {
    border-color: var(--up-border);
    background: var(--up-soft);
  }

  .status.error {
    border-color: var(--error-border);
    background: var(--error-soft);
  }

  .headline {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .words {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  strong {
    font-size: 15px;
  }

  .secondary {
    font-size: 12px;
    color: var(--secondary-text);
  }

  .retry .secondary {
    color: var(--retry-text);
  }

  .explanation {
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
  }

  dl {
    margin-top: 0;
    margin-bottom: 6px;
  }

  dt {
    color: var(--secondary-text);
  }

  dd {
    flex: 1;
    min-width: 0;
    margin: 0;
    color: var(--mono-text);
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
    -webkit-user-select: text;
  }

  .logs summary {
    justify-content: space-between;
    color: var(--secondary-text);
    cursor: pointer;
    list-style: none;
  }

  .logs summary::-webkit-details-marker {
    display: none;
  }

  .logs[open] summary svg {
    transform: rotate(90deg);
  }

  pre {
    max-height: 140px;
    margin: 0 12px;
    padding: 8px;
    overflow: auto;
    border-radius: 6px;
    background: var(--surface-strong);
    white-space: pre-wrap;
    word-break: break-all;
    user-select: text;
    -webkit-user-select: text;
  }

  .copy {
    margin: 6px 12px 10px;
    font-size: 12px;
  }

  .confirm {
    margin: 14px 12px 0;
    padding: 12px;
    border: 1px solid var(--error-border);
    border-radius: 10px;
    background: var(--error-soft);
  }

  .confirm p {
    margin: 0 0 10px;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>

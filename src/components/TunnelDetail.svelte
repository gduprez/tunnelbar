<script lang="ts">
  import {
    errorMessage,
    formatDuration,
    forwardSummary,
    isRunning,
    stateText,
    tunnelTarget,
    type Translate,
  } from '../lib/format';
  import type { TunnelView } from '../lib/types';
  import StatusDot from './StatusDot.svelte';

  let {
    tunnel,
    t,
    now,
    ontoggle,
    onedit,
    onduplicate,
    ondelete,
    oncopycommand,
    oncopylogs,
  }: {
    tunnel: TunnelView;
    t: Translate;
    now: number;
    ontoggle: () => void;
    onedit: () => void;
    onduplicate: () => void;
    ondelete: () => void;
    oncopycommand: () => void;
    oncopylogs: () => void;
  } = $props();

  let confirming = $state(false);
  const current = $derived(tunnel.status.state);
  const error = $derived(
    current.state === 'failed'
      ? current.error
      : current.state === 'reconnecting'
        ? current.last_error
        : null,
  );
</script>

<section class="detail">
  <div class="status">
    <StatusDot state={current} />
    <strong>{stateText(t, current)}</strong>
    {#if current.state === 'up' && tunnel.status.up_since_ms != null}
      <span class="muted">
        {t('detail.uptime', { duration: formatDuration(now - tunnel.status.up_since_ms) })}
      </span>
    {/if}
  </div>
  {#if error}
    <p class="error" role="alert">{errorMessage(t, error)}</p>
  {/if}
  {#if tunnel.status.reconnections > 0}
    <p class="muted">{t('detail.reconnections', { count: tunnel.status.reconnections })}</p>
  {/if}

  <h2>{t('detail.forwards')}</h2>
  <ul class="forwards">
    {#each tunnel.forward as forward, index (index)}
      <li>{forwardSummary(forward)}</li>
    {/each}
  </ul>
  <p class="muted">{t('detail.via', { target: tunnelTarget(tunnel) })}</p>

  <div class="actions">
    <button type="button" class="primary" onclick={ontoggle}>
      {isRunning(current) ? t('actions.stop') : t('actions.start')}
    </button>
    <button type="button" onclick={onedit}>{t('common.edit')}</button>
    <button type="button" onclick={onduplicate}>{t('common.duplicate')}</button>
    <button type="button" onclick={oncopycommand}>{t('actions.copy_command')}</button>
  </div>

  <div class="logs-header">
    <h2>{t('detail.logs')}</h2>
    {#if tunnel.status.logs.length > 0}
      <button type="button" class="link" onclick={oncopylogs}>{t('actions.copy_logs')}</button>
    {/if}
  </div>
  {#if tunnel.status.logs.length > 0}
    <pre class="logs">{tunnel.status.logs.join('\n')}</pre>
  {:else}
    <p class="muted">{t('detail.no_logs')}</p>
  {/if}

  <div class="danger-zone">
    {#if confirming}
      <p>{t('detail.delete_confirm', { name: tunnel.name })}</p>
      <button type="button" class="danger" onclick={ondelete}>{t('common.delete')}</button>
      <button type="button" onclick={() => (confirming = false)}>{t('common.cancel')}</button>
    {:else}
      <button type="button" class="link danger-text" onclick={() => (confirming = true)}>
        {t('common.delete')}
      </button>
    {/if}
  </div>
</section>

<style>
  .detail {
    padding: 0 16px 16px;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  h2 {
    margin: 14px 0 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    color: var(--secondary-text);
  }

  .forwards {
    margin: 0;
    padding-left: 16px;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 12px;
  }

  .muted {
    margin: 4px 0;
    color: var(--secondary-text);
  }

  .error {
    margin: 8px 0;
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--danger-background);
    color: var(--danger);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 12px;
  }

  .logs-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }

  .logs {
    max-height: 140px;
    margin: 0;
    padding: 8px;
    overflow: auto;
    border-radius: 6px;
    background: var(--code-background);
    font-size: 11px;
    white-space: pre-wrap;
    word-break: break-all;
    user-select: text;
    -webkit-user-select: text;
  }

  .danger-zone {
    margin-top: 16px;
  }

  .danger-zone p {
    margin: 0 0 6px;
  }
</style>

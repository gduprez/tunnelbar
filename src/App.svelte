<script lang="ts">
  import { onMount, tick } from 'svelte';
  import ActionMenu from './components/ActionMenu.svelte';
  import ConfigBanner from './components/ConfigBanner.svelte';
  import Settings from './components/Settings.svelte';
  import TunnelDetail from './components/TunnelDetail.svelte';
  import TunnelEditor from './components/TunnelEditor.svelte';
  import TrayMissing from './components/TrayMissing.svelte';
  import TunnelList from './components/TunnelList.svelte';
  import { api } from './lib/api';
  import { emptyForm, fromTunnel, type TunnelForm } from './lib/form';
  import { applyStatus, isRunning } from './lib/format';
  import { createTranslator } from './lib/i18n';
  import { catalogs } from './lib/locales';
  import type {
    ConfigInfo,
    ConfigProblem,
    LoginItem,
    PlatformView,
    SettingsView,
    Tunnel,
    TunnelView,
  } from './lib/types';
  import { listSummary } from './lib/view';

  let { locale }: { locale: string } = $props();

  type View =
    | { name: 'list' }
    | { name: 'detail'; id: string }
    | { name: 'edit'; form: TunnelForm; previousId: string | null }
    | { name: 'settings' };

  let view = $state<View>({ name: 'list' });
  let tunnels = $state<TunnelView[]>([]);
  let config = $state<ConfigInfo | null>(null);
  let settings = $state<SettingsView | null>(null);
  let loginItem = $state<LoginItem | null>(null);
  let platform = $state<PlatformView | null>(null);
  let now = $state(Date.now());
  let notice = $state<string | null>(null);
  let confirmingDelete = $state(false);

  let header = $state<HTMLElement>();
  let body = $state<HTMLElement>();
  let footer = $state<HTMLElement>();

  const t = $derived(createTranslator(catalogs, settings?.locale ?? locale));
  const summary = $derived(listSummary(t, tunnels));
  const shown = $derived.by(() => {
    if (view.name !== 'detail') return undefined;
    const id = view.id;
    return tunnels.find((tunnel) => tunnel.id === id);
  });
  const title = $derived.by(() => {
    switch (view.name) {
      case 'list':
        return t('panel.title');
      case 'detail':
        return shown?.name ?? '';
      case 'edit':
        return view.previousId === null ? t('form.title_new') : t('form.title_edit');
      case 'settings':
        return t('settings.title');
    }
  });

  async function refresh() {
    [tunnels, config] = await Promise.all([api.listTunnels(), api.configInfo()]);
  }

  onMount(() => {
    refresh();
    api.settings().then((loaded) => (settings = loaded));
    api.loginItem().then((loaded) => (loginItem = loaded));
    api.platform().then((loaded) => (platform = loaded));
    const unlisteners = [
      api.onStatus((status) => (tunnels = applyStatus(tunnels, status))),
      api.onTunnelsChanged(refresh),
    ];
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => {
      clearInterval(timer);
      for (const unlisten of unlisteners) unlisten.then((stop) => stop());
    };
  });

  // The panel takes the height of its content; Rust bounds it.
  $effect(() => {
    if (!header || !body || typeof ResizeObserver === 'undefined') return;
    let frame = 0;
    const fit = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const height =
          (header?.offsetHeight ?? 0) +
          (body?.firstElementChild?.scrollHeight ?? 0) +
          (footer?.offsetHeight ?? 0);
        api.fitPanel(height).catch(() => {});
      });
    };
    const observer = new ResizeObserver(fit);
    for (const element of [header, body, body.firstElementChild, footer]) {
      if (element) observer.observe(element);
    }
    fit();
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  });

  async function go(next: View) {
    confirmingDelete = false;
    view = next;
    await tick();
    if (body) body.scrollTop = 0;
  }

  function toggle(tunnel: TunnelView) {
    if (isRunning(tunnel.status.state)) api.stopTunnel(tunnel.id);
    else api.startTunnel(tunnel.id);
  }

  async function copy(text: string) {
    await api.copy(text);
    notice = t('actions.copied');
    setTimeout(() => (notice = null), 1500);
  }

  function edit(tunnel: TunnelView) {
    go({ name: 'edit', form: fromTunnel(tunnel), previousId: tunnel.id });
  }

  function duplicate(tunnel: TunnelView) {
    const form = fromTunnel(tunnel);
    form.id = '';
    form.name = t('form.copy_of', { name: tunnel.name });
    go({ name: 'edit', form, previousId: null });
  }

  async function remove(tunnel: TunnelView) {
    try {
      await api.deleteTunnel(tunnel.id);
      await refresh();
      go({ name: 'list' });
    } catch (problem) {
      config = { path: config?.path ?? '', problem: problem as ConfigProblem };
    }
  }

  async function saved(tunnel: Tunnel) {
    await refresh();
    go({ name: 'detail', id: tunnel.id });
  }

  async function chooseLoginItem(enabled: boolean) {
    try {
      loginItem = await api.setLoginItem(enabled);
    } catch (error) {
      notice = String(error);
      loginItem = await api.loginItem();
    }
  }

  async function chooseLanguage(language: string) {
    settings = await api.setLanguage(language);
  }

  function back() {
    go(
      view.name === 'edit' && view.previousId
        ? { name: 'detail', id: view.previousId }
        : { name: 'list' },
    );
  }
</script>

<main>
  <header bind:this={header}>
    {#if view.name !== 'list'}
      <button
        type="button"
        class="icon"
        aria-label={t('common.back')}
        title={t('common.back')}
        onclick={back}
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"><path d="M10 3 5 8l5 5" /></svg
        >
      </button>
    {/if}
    <h1>{title}</h1>
    {#if view.name === 'list' && summary.text}
      <p class="summary" class:alert={summary.alert}>{summary.text}</p>
    {/if}
    {#if view.name === 'detail' && shown}
      {@const tunnel = shown}
      <ActionMenu
        label={t('menu.more')}
        items={[
          { label: t('menu.edit'), onselect: () => edit(tunnel) },
          { label: t('common.duplicate'), onselect: () => duplicate(tunnel) },
          {
            label: t('actions.copy_command'),
            onselect: async () => copy(await api.shellCommand(tunnel.id)),
          },
          {
            label: t('menu.delete'),
            danger: true,
            separatorBefore: true,
            onselect: () => (confirmingDelete = true),
          },
        ]}
      />
    {/if}
  </header>

  <div class="body" bind:this={body}>
    <div>
      {#if platform?.os === 'linux' && platform.tray_available !== true && view.name === 'list'}
        <TrayMissing {t} />
      {/if}
      {#if config?.problem && view.name !== 'edit'}
        <ConfigBanner problem={config.problem} {t} onopen={() => api.openConfig()} />
      {/if}

      {#if view.name === 'list'}
        <TunnelList
          {tunnels}
          {t}
          {now}
          ontoggle={toggle}
          onopen={(tunnel) => go({ name: 'detail', id: tunnel.id })}
          onadd={() => go({ name: 'edit', form: emptyForm(), previousId: null })}
        />
      {:else if view.name === 'detail' && shown}
        {@const tunnel = shown}
        <TunnelDetail
          {tunnel}
          {t}
          {now}
          {confirmingDelete}
          ontoggle={() => toggle(tunnel)}
          ondelete={() => remove(tunnel)}
          oncanceldelete={() => (confirmingDelete = false)}
          oncopylogs={() => copy(tunnel.status.logs.join('\n'))}
        />
      {:else if view.name === 'edit'}
        {#key view}
          <TunnelEditor
            initial={view.form}
            previousId={view.previousId}
            {tunnels}
            {t}
            validate={api.validateTunnel}
            save={api.saveTunnel}
            oncancel={back}
            onsaved={saved}
          />
        {/key}
      {:else if view.name === 'settings' && settings}
        <Settings
          {settings}
          {loginItem}
          configPath={config?.path ?? ''}
          {t}
          onlanguage={chooseLanguage}
          onloginitem={chooseLoginItem}
          onopenconfig={() => api.openConfig()}
        />
      {/if}
    </div>
  </div>

  {#if view.name === 'list'}
    <footer bind:this={footer}>
      <button
        type="button"
        class="link new"
        onclick={() => go({ name: 'edit', form: emptyForm(), previousId: null })}
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 14 14"
          fill="none"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linecap="round"
          aria-hidden="true"><path d="M7 2v10M2 7h10" /></svg
        >
        {t('panel.new')}
      </button>
      <button
        type="button"
        class="icon"
        aria-label={t('panel.settings')}
        title={t('panel.settings')}
        onclick={() => go({ name: 'settings' })}
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
          ><circle cx="12" cy="12" r="3" /><path
            d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
          /></svg
        >
      </button>
    </footer>
  {/if}

  {#if notice}
    <div class="notice" role="status">{notice}</div>
  {/if}
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  header {
    display: flex;
    flex: none;
    align-items: center;
    gap: 4px;
    min-height: 36px;
    padding: 12px 8px 8px 16px;
  }

  header:has(> button:first-child) {
    padding-left: 8px;
  }

  h1 {
    flex: 1;
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .summary {
    margin: 0 8px 0 0;
    font-size: 12px;
    color: var(--secondary-text);
  }

  .summary.alert {
    color: var(--error-text);
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  footer {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: space-between;
    padding: 6px 8px;
    border-top: 1px solid var(--separator);
  }

  .new {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 10px;
    font-weight: 500;
  }

  .notice {
    position: fixed;
    bottom: 12px;
    left: 50%;
    transform: translateX(-50%);
    padding: 6px 12px;
    border-radius: 12px;
    background: var(--text);
    color: var(--background);
    font-size: 12px;
  }
</style>

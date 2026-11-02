<script lang="ts">
  import { onMount } from 'svelte';
  import ConfigBanner from './components/ConfigBanner.svelte';
  import Settings from './components/Settings.svelte';
  import TunnelDetail from './components/TunnelDetail.svelte';
  import TunnelEditor from './components/TunnelEditor.svelte';
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
    SettingsView,
    Tunnel,
    TunnelView,
  } from './lib/types';

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
  let now = $state(Date.now());
  let notice = $state<string | null>(null);

  const t = $derived(createTranslator(catalogs, settings?.locale ?? locale));
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
    view = { name: 'edit', form: fromTunnel(tunnel), previousId: tunnel.id };
  }

  function duplicate(tunnel: TunnelView) {
    const form = fromTunnel(tunnel);
    form.id = '';
    form.name = t('form.copy_of', { name: tunnel.name });
    view = { name: 'edit', form, previousId: null };
  }

  async function remove(tunnel: TunnelView) {
    try {
      await api.deleteTunnel(tunnel.id);
      await refresh();
      view = { name: 'list' };
    } catch (problem) {
      config = { path: config?.path ?? '', problem: problem as ConfigProblem };
    }
  }

  async function saved(tunnel: Tunnel) {
    await refresh();
    view = { name: 'detail', id: tunnel.id };
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
    view =
      view.name === 'edit' && view.previousId
        ? { name: 'detail', id: view.previousId }
        : { name: 'list' };
  }
</script>

<main>
  <header>
    {#if view.name !== 'list'}
      <button
        type="button"
        class="icon"
        aria-label={t('common.back')}
        title={t('common.back')}
        onclick={back}>‹</button
      >
    {/if}
    <h1>{title}</h1>
    {#if view.name === 'list'}
      <button
        type="button"
        class="icon"
        aria-label={t('panel.add')}
        title={t('panel.add')}
        onclick={() => (view = { name: 'edit', form: emptyForm(), previousId: null })}>+</button
      >
      <button
        type="button"
        class="icon"
        aria-label={t('panel.settings')}
        title={t('panel.settings')}
        onclick={() => (view = { name: 'settings' })}>⚙</button
      >
    {/if}
  </header>

  {#if config?.problem && view.name !== 'edit'}
    <ConfigBanner problem={config.problem} {t} onopen={() => api.openConfig()} />
  {/if}

  <div class="content">
    {#if view.name === 'list'}
      <TunnelList
        {tunnels}
        {t}
        ontoggle={toggle}
        onopen={(tunnel) => (view = { name: 'detail', id: tunnel.id })}
        onadd={() => (view = { name: 'edit', form: emptyForm(), previousId: null })}
      />
    {:else if view.name === 'detail' && shown}
      {@const tunnel = shown}
      <TunnelDetail
        {tunnel}
        {t}
        {now}
        ontoggle={() => toggle(tunnel)}
        onedit={() => edit(tunnel)}
        onduplicate={() => duplicate(tunnel)}
        ondelete={() => remove(tunnel)}
        oncopycommand={async () => copy(await api.shellCommand(tunnel.id))}
        oncopylogs={() => copy(tunnel.status.logs.join('\n'))}
      />
    {:else if view.name === 'edit'}
      {#key view}
        <TunnelEditor
          initial={view.form}
          previousId={view.previousId}
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
    align-items: center;
    gap: 4px;
    padding: 12px 10px 8px 16px;
  }

  h1 {
    flex: 1;
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .content {
    flex: 1;
    overflow-y: auto;
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

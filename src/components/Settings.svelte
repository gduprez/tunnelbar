<script lang="ts">
  import type { Translate } from '../lib/format';
  import type { LoginItem, SettingsView } from '../lib/types';
  import Toggle from './Toggle.svelte';

  let {
    settings,
    loginItem,
    configPath,
    t,
    onlanguage,
    onloginitem,
    onopenconfig,
  }: {
    settings: SettingsView;
    loginItem: LoginItem | null;
    configPath: string;
    t: Translate;
    onlanguage: (language: string) => void;
    onloginitem: (enabled: boolean) => void;
    onopenconfig: () => void;
  } = $props();

  const loginEnabled = $derived(loginItem === 'enabled' || loginItem === 'requires_approval');

  /** Each language named in itself, so that anyone can find theirs. */
  function nativeName(code: string): string {
    try {
      return new Intl.DisplayNames([code], { type: 'language' }).of(code) ?? code;
    } catch {
      return code;
    }
  }
</script>

<section class="settings">
  <h2 class="section-title">{t('settings.general')}</h2>
  <div class="group">
    <div class="group-row">
      <span id="setting-login" class="label">{t('settings.login')}</span>
      {#if loginItem !== null && loginItem !== 'unavailable'}
        <Toggle
          checked={loginEnabled}
          labelledby="setting-login"
          onchange={() => onloginitem(!loginEnabled)}
        />
      {/if}
    </div>
    <div class="group-row">
      <label for="setting-language" class="label">{t('settings.language')}</label>
      <select
        id="setting-language"
        value={settings.language}
        onchange={(event) => onlanguage((event.currentTarget as HTMLSelectElement).value)}
      >
        <option value="auto">{t('settings.language_auto')}</option>
        {#each settings.available as code (code)}
          <option value={code}>{nativeName(code)}</option>
        {/each}
      </select>
    </div>
  </div>
  {#if loginItem === 'requires_approval'}
    <p class="note">{t('settings.login_approval')}</p>
  {:else if loginItem === 'unavailable'}
    <p class="note">{t('settings.login_unavailable')}</p>
  {/if}

  <h2 class="section-title">{t('settings.config_file')}</h2>
  <div class="group file">
    <p class="mono path">{configPath}</p>
    <button type="button" class="link" onclick={onopenconfig}>{t('config.open_file')}</button>
  </div>
</section>

<style>
  .settings {
    padding-bottom: 16px;
  }

  .section-title:first-child {
    margin-top: 4px;
  }

  .label {
    flex: 1;
  }

  select {
    flex: none;
    width: 150px;
  }

  .note {
    margin: 6px 16px 0;
    font-size: 12px;
    color: var(--secondary-text);
  }

  .file {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 12px;
  }

  .path {
    margin: 0;
    color: var(--secondary-text);
    word-break: break-all;
    user-select: text;
    -webkit-user-select: text;
  }
</style>

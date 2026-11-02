<script lang="ts">
  import type { Translate } from '../lib/format';
  import type { LoginItem, SettingsView } from '../lib/types';

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
  <div>
    <label class="check">
      <input
        type="checkbox"
        checked={loginItem === 'enabled' || loginItem === 'requires_approval'}
        disabled={loginItem === null || loginItem === 'unavailable'}
        onchange={(event) => onloginitem((event.currentTarget as HTMLInputElement).checked)}
      />
      {t('settings.login')}
    </label>
    {#if loginItem === 'requires_approval'}
      <p class="note">{t('settings.login_approval')}</p>
    {:else if loginItem === 'unavailable'}
      <p class="note">{t('settings.login_unavailable')}</p>
    {/if}
  </div>

  <label>
    <span>{t('settings.language')}</span>
    <select
      value={settings.language}
      onchange={(event) => onlanguage((event.currentTarget as HTMLSelectElement).value)}
    >
      <option value="auto">{t('settings.language_auto')}</option>
      {#each settings.available as code (code)}
        <option value={code}>{nativeName(code)}</option>
      {/each}
    </select>
  </label>

  <div>
    <span class="label">{t('settings.config_file')}</span>
    <p class="path">{configPath}</p>
    <button type="button" onclick={onopenconfig}>{t('config.open_file')}</button>
  </div>
</section>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 0 16px 16px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  label > span,
  .label {
    font-size: 11px;
    font-weight: 600;
    color: var(--secondary-text);
  }

  label.check {
    flex-direction: row;
    align-items: center;
    gap: 6px;
  }

  .note {
    margin: 4px 0 0 22px;
    font-size: 11px;
    color: var(--secondary-text);
  }

  .path {
    margin: 4px 0 8px;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 11px;
    word-break: break-all;
    user-select: text;
    -webkit-user-select: text;
  }
</style>

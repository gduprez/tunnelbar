<script lang="ts">
  import { untrack } from 'svelte';
  import { fieldErrors, slugify, toTunnel, type TunnelForm } from '../lib/form';
  import { forwardSummary, validationMessage, type Translate } from '../lib/format';
  import type { ConfigProblem, Tunnel, ValidationError } from '../lib/types';

  let {
    initial,
    previousId,
    t,
    validate,
    save,
    oncancel,
    onsaved,
  }: {
    initial: TunnelForm;
    /** Id of the edited tunnel, null when creating one. */
    previousId: string | null;
    t: Translate;
    validate: (tunnel: Tunnel, previousId: string | null) => Promise<ValidationError[]>;
    save: (tunnel: Tunnel, previousId: string | null) => Promise<void>;
    oncancel: () => void;
    onsaved: (tunnel: Tunnel) => void;
  } = $props();

  let form = $state($state.snapshot(untrack(() => initial)));
  // A new tunnel's id follows its name until the user types an id.
  let idTouched = $state(untrack(() => previousId !== null || initial.id !== ''));
  let errors = $state<ValidationError[]>([]);
  let showErrors = $state(false);
  let saving = $state(false);
  let failure = $state<string | null>(null);

  const byField = $derived(fieldErrors(errors));

  $effect(() => {
    const tunnel = toTunnel(form);
    const timer = setTimeout(async () => {
      errors = await validate(tunnel, previousId);
    }, 200);
    return () => clearTimeout(timer);
  });

  function onNameInput() {
    if (!idTouched) form.id = slugify(form.name);
  }

  function messages(field: string): string[] {
    return showErrors || form.name !== ''
      ? (byField[field] ?? []).map((e) => validationMessage(t, e))
      : [];
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    const tunnel = toTunnel(form);
    errors = await validate(tunnel, previousId);
    showErrors = true;
    if (errors.length > 0) return;
    saving = true;
    failure = null;
    try {
      await save(tunnel, previousId);
      onsaved(tunnel);
    } catch (problem) {
      failure = describe(problem as ConfigProblem);
    } finally {
      saving = false;
    }
  }

  function describe(problem: ConfigProblem): string {
    switch (problem?.kind) {
      case 'invalid':
        errors = problem.errors;
        return t('form.fix_errors');
      case 'file_broken':
        return t('config.problem.file_broken');
      case 'io':
      case 'parse':
        return t('form.save_failed', { message: problem.message });
      default:
        return t('form.save_failed', { message: String(problem) });
    }
  }

  function addForward() {
    form.forwards.push({
      type: 'local',
      local_port: '',
      remote_host: '127.0.0.1',
      remote_port: '',
    });
  }
</script>

{#snippet fieldErrorsOf(field: string)}
  {#each messages(field) as message (message)}
    <span class="field-error">{message}</span>
  {/each}
{/snippet}

<form class="editor" onsubmit={submit} novalidate>
  <div class="field">
    <label for="tunnel-name">{t('form.name')}</label>
    <input
      id="tunnel-name"
      bind:value={form.name}
      oninput={onNameInput}
      placeholder="PostgreSQL prod"
    />
    {@render fieldErrorsOf('name')}
  </div>

  <div class="field">
    <label for="tunnel-host">{t('form.host')}</label>
    <input
      id="tunnel-host"
      bind:value={form.host}
      placeholder="203.0.113.10"
      autocapitalize="off"
      spellcheck="false"
    />
    <small>{t('form.host_hint')}</small>
    {@render fieldErrorsOf('host')}
  </div>

  <div class="row">
    <div class="field">
      <label for="tunnel-user">{t('form.user')}</label>
      <input
        id="tunnel-user"
        bind:value={form.user}
        placeholder={t('form.optional')}
        autocapitalize="off"
        spellcheck="false"
      />
      {@render fieldErrorsOf('user')}
    </div>
    <div class="field narrow">
      <label for="tunnel-port">{t('form.port')}</label>
      <input id="tunnel-port" bind:value={form.port} placeholder="22" inputmode="numeric" />
      {@render fieldErrorsOf('port')}
    </div>
  </div>

  <fieldset>
    <legend>{t('form.forwards')}</legend>
    {#each form.forwards as forward, index (index)}
      {#if forward.type === 'local'}
        <div class="row">
          <div class="field narrow">
            <label for="local-port-{index}">{t('form.local_port')}</label>
            <input
              id="local-port-{index}"
              bind:value={forward.local_port}
              placeholder="54321"
              inputmode="numeric"
            />
          </div>
          <span class="arrow">→</span>
          <div class="field">
            <label for="remote-host-{index}">{t('form.remote_host')}</label>
            <input
              id="remote-host-{index}"
              bind:value={forward.remote_host}
              autocapitalize="off"
              spellcheck="false"
            />
          </div>
          <div class="field narrow">
            <label for="remote-port-{index}">{t('form.remote_port')}</label>
            <input
              id="remote-port-{index}"
              bind:value={forward.remote_port}
              placeholder="5432"
              inputmode="numeric"
            />
          </div>
          <button
            type="button"
            class="icon remove"
            aria-label={t('form.remove_forward')}
            title={t('form.remove_forward')}
            onclick={() => form.forwards.splice(index, 1)}>×</button
          >
        </div>
      {:else}
        <p class="other">
          {t('form.other_forward', { summary: forwardSummary(toTunnel(form).forward[index]!) })}
        </p>
      {/if}
      {@render fieldErrorsOf(`forward.${index}.local_port`)}
      {@render fieldErrorsOf(`forward.${index}.host`)}
      {@render fieldErrorsOf(`forward.${index}.remote_port`)}
    {/each}
    {@render fieldErrorsOf('forwards')}
    <button type="button" class="link" onclick={addForward}>{t('form.add_forward')}</button>
  </fieldset>

  <label class="check">
    <input type="checkbox" bind:checked={form.auto_reconnect} />
    {t('form.auto_reconnect')}
  </label>
  <label class="check">
    <input type="checkbox" bind:checked={form.autostart} />
    {t('form.autostart')}
  </label>
  <label class="check">
    <input type="checkbox" bind:checked={form.accept_new_host_key} />
    {t('form.accept_new_host_key')}
  </label>

  <details>
    <summary>{t('form.advanced')}</summary>
    <div class="field">
      <label for="tunnel-id">{t('form.id')}</label>
      <input
        id="tunnel-id"
        bind:value={form.id}
        oninput={() => (idTouched = true)}
        autocapitalize="off"
        spellcheck="false"
      />
      {@render fieldErrorsOf('id')}
    </div>
  </details>
  {@render fieldErrorsOf('form')}

  {#if failure}
    <p class="failure" role="alert">{failure}</p>
  {/if}

  <div class="buttons">
    <button type="button" onclick={oncancel}>{t('common.cancel')}</button>
    <button type="submit" class="primary" disabled={saving}>{t('common.save')}</button>
  </div>
</form>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 0 16px 16px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: 1;
    min-width: 0;
  }

  .field > label {
    font-size: 11px;
    font-weight: 600;
    color: var(--secondary-text);
  }

  small {
    color: var(--secondary-text);
    font-size: 11px;
  }

  .row {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }

  .narrow {
    flex: 0 0 72px;
  }

  .arrow {
    padding-top: 20px;
    color: var(--secondary-text);
  }

  .remove {
    margin-top: 18px;
  }

  fieldset {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  legend {
    padding: 0 4px;
    font-size: 11px;
    font-weight: 600;
    color: var(--secondary-text);
  }

  fieldset > .link {
    align-self: flex-start;
  }

  .other {
    margin: 0;
    font-size: 12px;
    color: var(--secondary-text);
  }

  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }

  details summary {
    cursor: pointer;
    color: var(--secondary-text);
    font-size: 12px;
  }

  .field-error,
  .failure {
    color: var(--danger);
    font-size: 11px;
  }

  .failure {
    margin: 0;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>

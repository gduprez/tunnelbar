<script lang="ts">
  import { untrack } from 'svelte';
  import { fieldErrors, slugify, toTunnel, type TunnelForm } from '../lib/form';
  import { forwardSummary, validationMessage, type Translate } from '../lib/format';
  import type { ConfigProblem, Tunnel, ValidationError } from '../lib/types';
  import { localPorts, suggestPort } from '../lib/view';
  import Toggle from './Toggle.svelte';

  let {
    initial,
    previousId,
    tunnels,
    t,
    validate,
    save,
    oncancel,
    onsaved,
  }: {
    initial: TunnelForm;
    /** Id of the edited tunnel, null when creating one. */
    previousId: string | null;
    /** Every configured tunnel, in configuration order. */
    tunnels: Tunnel[];
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

  /** Validation message, naming the other tunnel of a port clash and suggesting a free port. */
  function describeError(error: ValidationError): string {
    if (error.code !== 'local_port_conflict' || error.port == null) {
      return validationMessage(t, error);
    }
    const other = error.other_tunnel == null ? undefined : tunnels[error.other_tunnel];
    const sameTunnel = other === undefined || other.id === previousId;
    const message = sameTunnel
      ? validationMessage(t, error)
      : t('validation.local_port_conflict_named', { port: error.port, name: other.name });
    const free = suggestPort(
      [...localPorts(tunnels, previousId), ...localPorts([toTunnel(form)], null)],
      error.port,
    );
    return free == null ? message : `${message} ${t('form.try_port', { port: free })}`;
  }

  function messages(field: string): string[] {
    return showErrors || form.name !== '' ? (byField[field] ?? []).map(describeError) : [];
  }

  function invalid(field: string): boolean {
    return messages(field).length > 0;
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
      failure = describeProblem(problem as ConfigProblem);
    } finally {
      saving = false;
    }
  }

  function describeProblem(problem: ConfigProblem): string {
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
    <p class="field-error">{message}</p>
  {/each}
{/snippet}

<form class="editor" onsubmit={submit} novalidate>
  <div class="fields">
    <h2 class="section-title">{t('form.section_connection')}</h2>
    <div class="group">
      <div class="group-row">
        <label for="tunnel-name">{t('form.name')}</label>
        <input
          id="tunnel-name"
          class="plain"
          bind:value={form.name}
          oninput={onNameInput}
          placeholder="PostgreSQL prod"
          aria-invalid={invalid('name')}
        />
      </div>
      <div class="group-row">
        <label for="tunnel-host">{t('form.host')}</label>
        <input
          id="tunnel-host"
          class="plain mono"
          bind:value={form.host}
          placeholder={t('form.host_hint')}
          autocapitalize="off"
          spellcheck="false"
          aria-invalid={invalid('host')}
        />
      </div>
      <div class="group-row">
        <label for="tunnel-user">{t('form.user')}</label>
        <input
          id="tunnel-user"
          class="plain"
          bind:value={form.user}
          placeholder={t('form.optional')}
          autocapitalize="off"
          spellcheck="false"
          aria-invalid={invalid('user')}
        />
        <label for="tunnel-port" class="inline">{t('form.port')}</label>
        <input
          id="tunnel-port"
          class="plain mono port"
          bind:value={form.port}
          placeholder="22"
          inputmode="numeric"
          aria-invalid={invalid('port')}
        />
      </div>
    </div>
    <div class="errors">
      {@render fieldErrorsOf('name')}
      {@render fieldErrorsOf('host')}
      {@render fieldErrorsOf('user')}
      {@render fieldErrorsOf('port')}
    </div>

    <h2 class="section-title">{t('form.forwards')}</h2>
    <div class="group forwards">
      {#each form.forwards as forward, index (index)}
        {#if forward.type === 'local'}
          <div class="forward">
            <label for="local-port-{index}">{t('form.local_port')}</label>
            <span></span>
            <label for="remote-host-{index}">{t('form.remote_host')}</label>
            <label for="remote-port-{index}">{t('form.remote_port')}</label>
            <span></span>
            <input
              id="local-port-{index}"
              class="mono"
              bind:value={forward.local_port}
              placeholder="54321"
              inputmode="numeric"
              aria-invalid={invalid(`forward.${index}.local_port`)}
            />
            <svg
              width="16"
              height="16"
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"><path d="M3 8h10M9.5 4.5 13 8l-3.5 3.5" /></svg
            >
            <input
              id="remote-host-{index}"
              class="mono"
              bind:value={forward.remote_host}
              autocapitalize="off"
              spellcheck="false"
              aria-invalid={invalid(`forward.${index}.host`)}
            />
            <input
              id="remote-port-{index}"
              class="mono"
              bind:value={forward.remote_port}
              placeholder="5432"
              inputmode="numeric"
              aria-invalid={invalid(`forward.${index}.remote_port`)}
            />
            <button
              type="button"
              class="icon"
              aria-label={t('form.remove_forward')}
              title={t('form.remove_forward')}
              onclick={() => form.forwards.splice(index, 1)}
            >
              <svg
                width="12"
                height="12"
                viewBox="0 0 12 12"
                fill="none"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linecap="round"
                aria-hidden="true"><path d="m3 3 6 6M9 3 3 9" /></svg
              >
            </button>
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
      <button type="button" class="link add" onclick={addForward}>{t('form.add_forward')}</button>
    </div>

    <h2 class="section-title">{t('form.section_behaviour')}</h2>
    <div class="group">
      <div class="group-row">
        <span id="option-reconnect" class="option">{t('form.auto_reconnect')}</span>
        <Toggle
          checked={form.auto_reconnect}
          labelledby="option-reconnect"
          onchange={() => (form.auto_reconnect = !form.auto_reconnect)}
        />
      </div>
      <div class="group-row">
        <span id="option-autostart" class="option">{t('form.autostart')}</span>
        <Toggle
          checked={form.autostart}
          labelledby="option-autostart"
          onchange={() => (form.autostart = !form.autostart)}
        />
      </div>
    </div>

    <details class="advanced">
      <summary>{t('form.advanced')}</summary>
      <div class="group">
        <div class="group-row">
          <span id="option-host-key" class="option">{t('form.accept_new_host_key')}</span>
          <Toggle
            checked={form.accept_new_host_key}
            labelledby="option-host-key"
            onchange={() => (form.accept_new_host_key = !form.accept_new_host_key)}
          />
        </div>
        <div class="group-row">
          <label for="tunnel-id">{t('form.id')}</label>
          <input
            id="tunnel-id"
            class="plain mono"
            bind:value={form.id}
            oninput={() => (idTouched = true)}
            autocapitalize="off"
            spellcheck="false"
            aria-invalid={invalid('id')}
          />
        </div>
      </div>
      <div class="errors">{@render fieldErrorsOf('id')}</div>
    </details>
    <div class="errors">{@render fieldErrorsOf('form')}</div>

    {#if failure}
      <p class="failure" role="alert">{failure}</p>
    {/if}
  </div>

  <footer>
    <button type="button" onclick={oncancel}>{t('common.cancel')}</button>
    <button type="submit" class="primary" disabled={saving}>{t('common.save')}</button>
  </footer>
</form>

<style>
  .editor {
    display: flex;
    flex-direction: column;
  }

  .fields {
    padding-bottom: 12px;
  }

  .section-title:first-child {
    margin-top: 4px;
  }

  .group-row > label {
    flex: none;
    width: 82px;
    color: var(--secondary-text);
  }

  .group-row > label.inline {
    width: auto;
  }

  input.plain {
    flex: 1;
    height: 28px;
    padding: 0;
    border: none;
    border-radius: 0;
    background: none;
  }

  input.plain[aria-invalid='true'] {
    background: none;
    color: var(--error-text);
  }

  input.port {
    flex: none;
    width: 48px;
    text-align: right;
  }

  .forwards {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
  }

  .forward {
    display: grid;
    grid-template-columns: 76px 16px minmax(0, 1fr) 64px 24px;
    gap: 4px 8px;
    align-items: center;
    color: var(--secondary-text);
  }

  .forward label {
    font-size: 11px;
  }

  .forward .icon {
    width: 24px;
    height: 24px;
  }

  .other {
    margin: 0;
    font-size: 12px;
    color: var(--secondary-text);
  }

  .add {
    align-self: flex-start;
    font-weight: 500;
  }

  .option {
    flex: 1;
  }

  .advanced {
    margin-top: 10px;
  }

  .advanced summary {
    margin: 0 16px 6px;
    padding: 6px 0;
    color: var(--secondary-text);
    cursor: pointer;
  }

  .errors {
    margin: 0 16px;
  }

  .field-error,
  .failure {
    margin: 6px 0 0;
    font-size: 12px;
    line-height: 1.4;
    color: var(--error-text);
  }

  .failure {
    margin: 10px 16px 0;
  }

  footer {
    position: sticky;
    bottom: 0;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 12px;
    border-top: 1px solid var(--separator);
    background: var(--background);
  }
</style>

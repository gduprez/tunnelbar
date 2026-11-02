import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { TunnelStatus, TunnelView } from './lib/types';

const listeners: { status?: (status: TunnelStatus) => void; changed?: () => void } = {};

vi.mock('./lib/api', () => ({
  api: {
    listTunnels: vi.fn(),
    startTunnel: vi.fn(async () => {}),
    stopTunnel: vi.fn(async () => {}),
    shellCommand: vi.fn(async () => '/usr/bin/ssh -N -- host'),
    configInfo: vi.fn(),
    validateTunnel: vi.fn(async () => []),
    saveTunnel: vi.fn(async () => {}),
    deleteTunnel: vi.fn(async () => {}),
    openConfig: vi.fn(async () => {}),
    settings: vi.fn(async () => ({ language: 'auto', locale: 'en', available: ['en', 'fr'] })),
    setLanguage: vi.fn(async (language: string) => ({
      language,
      locale: language === 'auto' ? 'en' : language,
      available: ['en', 'fr'],
    })),
    onStatus: vi.fn(async (callback: (status: TunnelStatus) => void) => {
      listeners.status = callback;
      return () => {};
    }),
    onTunnelsChanged: vi.fn(async (callback: () => void) => {
      listeners.changed = callback;
      return () => {};
    }),
    copy: vi.fn(async () => {}),
    fitPanel: vi.fn(async () => {}),
    loginItem: vi.fn(async () => 'disabled'),
    setLoginItem: vi.fn(async (enabled: boolean) => (enabled ? 'enabled' : 'disabled')),
  },
}));

const { api } = await import('./lib/api');
const App = (await import('./App.svelte')).default;

function view(state: TunnelStatus['state'] = { state: 'stopped' }): TunnelView {
  return {
    id: 'prod',
    name: 'PostgreSQL prod',
    host: '203.0.113.10',
    autostart: false,
    auto_reconnect: true,
    accept_new_host_key: false,
    forward: [{ type: 'local', local_port: 54321, remote_host: '127.0.0.1', remote_port: 5432 }],
    status: { id: 'prod', state, up_since_ms: null, reconnections: 0, logs: [] },
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.listTunnels).mockResolvedValue([view()]);
  vi.mocked(api.configInfo).mockResolvedValue({ path: '/x/tunnels.toml', problem: null });
  vi.mocked(api.validateTunnel).mockResolvedValue([]);
});

describe('App', () => {
  it('shows the empty state in the requested locale', async () => {
    vi.mocked(api.listTunnels).mockResolvedValue([]);
    vi.mocked(api.settings).mockResolvedValueOnce({
      language: 'auto',
      locale: 'fr',
      available: ['en', 'fr'],
    });
    render(App, { locale: 'fr' });
    expect(screen.getByRole('heading', { name: 'Tunnels SSH' })).toBeInTheDocument();
    expect(await screen.findByText("Aucun tunnel configuré pour l'instant.")).toBeInTheDocument();
  });

  it('lists tunnels and starts one with its switch', async () => {
    render(App, { locale: 'en' });
    const toggle = await screen.findByRole('switch', { name: 'Start or stop PostgreSQL prod' });
    expect(toggle).toHaveAttribute('aria-checked', 'false');
    await fireEvent.click(toggle);
    expect(api.startTunnel).toHaveBeenCalledWith('prod');
  });

  it('stops a running tunnel', async () => {
    vi.mocked(api.listTunnels).mockResolvedValue([view({ state: 'up' })]);
    render(App, { locale: 'en' });
    await fireEvent.click(await screen.findByRole('switch'));
    expect(api.stopTunnel).toHaveBeenCalledWith('prod');
  });

  it('follows live status updates', async () => {
    render(App, { locale: 'en' });
    const toggle = await screen.findByRole('switch');
    listeners.status!({
      id: 'prod',
      state: { state: 'failed', error: { code: 'auth_failed' } },
      up_since_ms: null,
      reconnections: 0,
      logs: [],
    });
    await waitFor(() => expect(screen.getByText('Authentication refused')).toBeInTheDocument());
    expect(screen.getByText('Needs attention: 1')).toBeInTheDocument();
    expect(toggle).toHaveAttribute('aria-checked', 'false');
  });

  it('shows the detail of a tunnel with its error and logs', async () => {
    const failed = view({ state: 'failed', error: { code: 'port_in_use', port: 54321 } });
    failed.status.logs = ['bind [127.0.0.1]:54321: Address already in use'];
    vi.mocked(api.listTunnels).mockResolvedValue([failed]);
    render(App, { locale: 'en' });

    expect(await screen.findByText(/Local port 54321 is already used/)).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'See the ssh output' }));
    expect(screen.getByRole('alert')).toHaveTextContent('Local port 54321 is already used');
    expect(screen.getByText('bind [127.0.0.1]:54321: Address already in use')).toBeInTheDocument();
    expect(screen.getByText('localhost:54321')).toBeInTheDocument();
    expect(screen.getByText('127.0.0.1:5432')).toBeInTheDocument();
  });

  it('copies the equivalent ssh command', async () => {
    render(App, { locale: 'en' });
    await fireEvent.click(await screen.findByText('PostgreSQL prod'));
    await fireEvent.click(screen.getByRole('button', { name: 'More actions' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Copy ssh command' }));
    await waitFor(() => expect(api.copy).toHaveBeenCalledWith('/usr/bin/ssh -N -- host'));
  });

  it('deletes a tunnel after confirmation', async () => {
    render(App, { locale: 'en' });
    await fireEvent.click(await screen.findByText('PostgreSQL prod'));
    await fireEvent.click(screen.getByRole('button', { name: 'More actions' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Delete…' }));
    expect(api.deleteTunnel).not.toHaveBeenCalled();
    expect(
      screen.getByText('Delete “PostgreSQL prod”? This cannot be undone.'),
    ).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
    expect(api.deleteTunnel).toHaveBeenCalledWith('prod');
  });

  it('creates a tunnel with an id derived from its name', async () => {
    render(App, { locale: 'en' });
    await fireEvent.click(await screen.findByRole('button', { name: 'New tunnel' }));
    await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'Redis staging' } });
    await fireEvent.input(screen.getByLabelText('Server'), { target: { value: 'staging' } });
    await fireEvent.input(screen.getByLabelText('Port on this Mac'), {
      target: { value: '63790' },
    });
    await fireEvent.input(
      screen.getByLabelText('Port', { exact: true, selector: '#remote-port-0' }),
      {
        target: { value: '6379' },
      },
    );
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => expect(api.saveTunnel).toHaveBeenCalled());
    expect(vi.mocked(api.saveTunnel).mock.calls[0]).toEqual([
      {
        id: 'redis-staging',
        name: 'Redis staging',
        host: 'staging',
        autostart: false,
        auto_reconnect: true,
        accept_new_host_key: false,
        forward: [
          { type: 'local', local_port: 63790, remote_host: '127.0.0.1', remote_port: 6379 },
        ],
      },
      null,
    ]);
  });

  it('shows validation errors and does not save', async () => {
    vi.mocked(api.validateTunnel).mockResolvedValue([
      { tunnel: 1, forward: null, code: 'invalid_host' },
    ]);
    render(App, { locale: 'en' });
    await fireEvent.click(await screen.findByRole('button', { name: 'New tunnel' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    expect(
      await screen.findByText(/Enter a host name, an IP address or an alias/),
    ).toBeInTheDocument();
    expect(api.saveTunnel).not.toHaveBeenCalled();
  });

  it('edits a tunnel in place', async () => {
    render(App, { locale: 'en' });
    await fireEvent.click(await screen.findByText('PostgreSQL prod'));
    await fireEvent.click(screen.getByRole('button', { name: 'More actions' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Edit…' }));
    await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'Prod DB' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => expect(api.saveTunnel).toHaveBeenCalled());
    const [tunnel, previousId] = vi.mocked(api.saveTunnel).mock.calls[0]!;
    expect(tunnel.id).toBe('prod');
    expect(tunnel.name).toBe('Prod DB');
    expect(previousId).toBe('prod');
  });

  it('shows configuration problems with a way to open the file', async () => {
    vi.mocked(api.configInfo).mockResolvedValue({
      path: '/x/tunnels.toml',
      problem: { kind: 'parse', message: 'expected `]`', line: 3 },
    });
    render(App, { locale: 'en' });
    expect(await screen.findByRole('alert')).toHaveTextContent('line 3: expected `]`');
    await fireEvent.click(screen.getByRole('button', { name: 'Open the configuration file' }));
    expect(api.openConfig).toHaveBeenCalled();
  });

  it('reloads the list when the configuration changes', async () => {
    render(App, { locale: 'en' });
    await screen.findByText('PostgreSQL prod');
    vi.mocked(api.listTunnels).mockResolvedValue([]);
    listeners.changed!();
    expect(await screen.findByText('No tunnel configured yet.')).toBeInTheDocument();
  });

  it('switches the language from the settings', async () => {
    render(App, { locale: 'en' });
    await fireEvent.click(await screen.findByRole('button', { name: 'Settings' }));
    await fireEvent.change(screen.getByLabelText('Language'), { target: { value: 'fr' } });
    expect(api.setLanguage).toHaveBeenCalledWith('fr');
    expect(await screen.findByRole('heading', { name: 'Réglages' })).toBeInTheDocument();
  });

  it('turns launch at login on from the settings', async () => {
    render(App, { locale: 'en' });
    await fireEvent.click(await screen.findByRole('button', { name: 'Settings' }));
    const checkbox = await screen.findByLabelText('Open TunnelBar at login');
    expect(checkbox).not.toBeChecked();
    await fireEvent.click(checkbox);
    expect(api.setLoginItem).toHaveBeenCalledWith(true);
    await waitFor(() => expect(checkbox).toBeChecked());
  });

  it('explains when macOS needs the user to approve the login item', async () => {
    vi.mocked(api.loginItem).mockResolvedValueOnce('requires_approval');
    render(App, { locale: 'en' });
    await fireEvent.click(await screen.findByRole('button', { name: 'Settings' }));
    expect(await screen.findByText(/System Settings › General › Login Items/)).toBeInTheDocument();
  });

  it('summarises running tunnels in the header', async () => {
    vi.mocked(api.listTunnels).mockResolvedValue([view({ state: 'up' }), { ...view(), id: 'b' }]);
    render(App, { locale: 'en' });
    expect(await screen.findByText('1 of 2 running')).toBeInTheDocument();
  });

  it('closes the actions menu with Escape and gives the focus back', async () => {
    render(App, { locale: 'en' });
    await fireEvent.click(await screen.findByText('PostgreSQL prod'));
    const trigger = screen.getByRole('button', { name: 'More actions' });
    await fireEvent.click(trigger);
    const menu = screen.getByRole('menu');
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    await fireEvent.keyDown(menu, { key: 'Escape' });
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it('names the other tunnel of a port clash and suggests a free port', async () => {
    vi.mocked(api.listTunnels).mockResolvedValue([view()]);
    vi.mocked(api.validateTunnel).mockResolvedValue([
      {
        tunnel: 1,
        forward: 0,
        code: 'local_port_conflict',
        port: 54321,
        other_tunnel: 0,
        other_forward: 0,
      },
    ]);
    render(App, { locale: 'en' });
    await fireEvent.click(await screen.findByRole('button', { name: 'New tunnel' }));
    await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'Other' } });
    expect(
      await screen.findByText('Port 54321 is already used by “PostgreSQL prod”. Try 54322.'),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Port on this Mac')).toHaveAttribute('aria-invalid', 'true');
  });
});

import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Diagnostics, OpenRequest, Profile, Rule, Snapshot } from './types';
import { validateRequest } from './validation';

export const isPreview = !isTauri();
const key = 'epf-preview-v1';
const timestamp = () => Math.floor(Date.now() / 1000);
const network: Snapshot['network'] = {
  localIp: '192.168.1.24',
  gatewayIp: '192.168.1.1',
  interface: 'Wi-Fi',
  routerName: 'Home router',
  manufacturer: 'Sample manufacturer',
  model: 'Preview device',
  routerId: 'preview-router',
  wanIp: '203.0.113.42',
  upnpAvailable: true,
  message: 'Sample UPnP network for the browser preview.',
  checkedAt: timestamp(),
};
function sampleRule(
  id: string,
  name: string,
  port: number,
  protocol: 'tcp' | 'udp',
  status: Rule['status'],
): Rule {
  return {
    id,
    name,
    externalPort: port,
    internalPort: port,
    protocol,
    status,
    destinationIp: '192.168.1.24',
    leaseSeconds: 3600,
    createdAt: timestamp(),
    routerId: 'preview-router',
    description: `EPF:${id}`,
  };
}
function readPreview(): Snapshot {
  let saved: Pick<Snapshot, 'rules' | 'profiles'> | null = null;
  try {
    saved = JSON.parse(localStorage.getItem(key) ?? 'null');
  } catch {
    /* Reset only disposable preview data. */
  }
  const offline = new URLSearchParams(location.search).get('scenario') === 'offline';
  const rules = saved?.rules ?? [
    sampleRule('sample-minecraft', 'Minecraft server', 25565, 'tcp', 'active'),
    sampleRule('sample-valheim', 'Valheim', 2456, 'udp', 'inactive'),
  ];
  return {
    network: {
      ...network,
      checkedAt: timestamp(),
      upnpAvailable: !offline,
      message: offline
        ? 'No UPnP router responded. Check your connection and router settings.'
        : network.message,
    },
    rules: offline ? rules.map((r) => ({ ...r, status: 'unknown' as const })) : rules,
    profiles: saved?.profiles ?? [],
    warning: offline ? 'Automatic forwarding is unavailable on this sample network.' : null,
  };
}
function writePreview(snapshot: Snapshot) {
  localStorage.setItem(key, JSON.stringify({ rules: snapshot.rules, profiles: snapshot.profiles }));
}

let snapshotInFlight: Promise<Snapshot> | null = null;

export const api = {
  async snapshot(): Promise<Snapshot> {
    if (!snapshotInFlight) {
      snapshotInFlight = (
        isPreview ? Promise.resolve(readPreview()) : invoke<Snapshot>('get_snapshot')
      ).finally(() => {
        snapshotInFlight = null;
      });
    }
    return snapshotInFlight;
  },
  async open(request: OpenRequest, confirmed: boolean): Promise<Rule[]> {
    if (!confirmed) throw new Error('Confirm this operation before opening a port.');
    const validation = validateRequest(request);
    if (validation) throw new Error(validation);
    if (!isPreview) return invoke('open_ports', { request, confirmed });
    const state = readPreview();
    if (!state.network.upnpAvailable) throw new Error(state.network.message);
    const protocols = request.protocol === 'both' ? (['tcp', 'udp'] as const) : [request.protocol];
    if (
      state.rules.some(
        (r) =>
          r.externalPort === request.externalPort &&
          protocols.includes(r.protocol) &&
          r.status === 'active',
      )
    )
      throw new Error('This port already has an active rule. Choose another port.');
    const rules = protocols.map((protocol) => ({
      ...request,
      protocol,
      id: crypto.randomUUID(),
      destinationIp: request.destinationIp ?? network.localIp!,
      createdAt: timestamp(),
      routerId: 'preview-router',
      description: 'EPF:preview',
      status: 'active' as const,
    }));
    state.rules.push(...rules);
    writePreview(state);
    return rules;
  },
  async close(id: string, confirmed: boolean): Promise<void> {
    if (!confirmed) throw new Error('Confirm this operation before closing a port.');
    if (!isPreview) return invoke('close_rule', { id, confirmed });
    const state = readPreview();
    if (!state.network.upnpAvailable) throw new Error(state.network.message);
    state.rules = state.rules.map((rule) =>
      rule.id === id ? { ...rule, status: 'inactive' } : rule,
    );
    writePreview(state);
  },
  async saveProfile(request: OpenRequest): Promise<Profile> {
    const validation = validateRequest(request);
    if (validation) throw new Error(validation);
    if (!isPreview) return invoke('save_profile', { request });
    const state = readPreview();
    const profile = { id: crypto.randomUUID(), request };
    state.profiles.push(profile);
    writePreview(state);
    return profile;
  },
  async deleteProfile(id: string): Promise<void> {
    if (!isPreview) return invoke('delete_profile', { id });
    const state = readPreview();
    state.profiles = state.profiles.filter((p) => p.id !== id);
    writePreview(state);
  },
  async diagnose(includePublicIp: boolean): Promise<Diagnostics> {
    if (!isPreview) return invoke('diagnose', { includePublicIp });
    return {
      network: readPreview().network,
      publicIp: includePublicIp ? '203.0.113.42' : null,
      nat: includePublicIp ? 'public' : 'unknown',
      findings: [
        includePublicIp
          ? 'Sample result: the WAN and public IP addresses match. Internet access to individual ports is still unverified.'
          : 'Allow a public IP lookup to compare it with your router’s WAN address.',
        'Keep your server running and allow incoming connections in your computer’s firewall.',
      ],
      reachability: 'Not tested. No external port-testing service is configured in this release.',
    };
  },
};

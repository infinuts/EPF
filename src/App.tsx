import { useCallback, useEffect, useState } from 'react';
import {
  ArrowRight,
  ArrowUpRight,
  ArrowsClockwise,
  BookmarkSimple,
  Check,
  CheckCircle,
  Circle,
  Copy,
  Desktop,
  GlobeHemisphereWest,
  Info,
  MagnifyingGlass,
  Plus,
  PlugsConnected,
  Question,
  Broadcast as Router,
  ShieldCheck,
  TerminalWindow,
  Trash,
  WarningCircle,
  WifiHigh,
  X,
} from '@phosphor-icons/react';
import { api, isPreview } from './api';
import type { Diagnostics, OpenRequest, Rule, Snapshot } from './types';
import RuleDialog from './components/RuleDialog';
import CloseDialog from './components/CloseDialog';
import { usePageMotion } from './usePageMotion';

type Page = 'ports' | 'diagnostics' | 'profiles' | 'guide';
type Filter = 'all' | 'active' | 'inactive';
const nav = [
  { id: 'ports', label: 'Port forwarding', icon: PlugsConnected },
  { id: 'diagnostics', label: 'Network diagnostics', icon: GlobeHemisphereWest },
  { id: 'profiles', label: 'Saved profiles', icon: BookmarkSimple },
] as const;
const errorMessage = (error: unknown) => (error instanceof Error ? error.message : String(error));
const date = (seconds: number) =>
  new Date(seconds * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
const protocolLabel = (protocol: string) =>
  protocol === 'both' ? 'TCP + UDP' : protocol.toUpperCase();

export default function App() {
  const [page, setPage] = useState<Page>('ports');
  const motionScope = usePageMotion(page);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [filter, setFilter] = useState<Filter>('all');
  const [search, setSearch] = useState('');
  const [dialog, setDialog] = useState<{ initial?: OpenRequest } | null>(null);
  const [closing, setClosing] = useState<Rule | null>(null);
  const [diagnostics, setDiagnostics] = useState<Diagnostics | null>(null);
  const [diagnosing, setDiagnosing] = useState(false);
  const [includePublic, setIncludePublic] = useState(false);
  const [copied, setCopied] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setSnapshot(await api.snapshot());
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setLoading(false);
    }
  }, []);
  useEffect(() => {
    let alive = true;
    void api
      .snapshot()
      .then((value) => {
        if (alive) setSnapshot(value);
      })
      .catch((e: unknown) => {
        if (alive) setError(errorMessage(e));
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, []);
  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 7000);
    return () => window.clearTimeout(timer);
  }, [toast]);
  const complete = (message: string) => {
    setToast(message);
    void refresh();
  };
  const network = snapshot?.network;
  const rules = snapshot?.rules ?? [];
  const active = rules.filter((r) => r.status === 'active').length;
  const filtered = rules.filter(
    (r) =>
      (filter === 'all' || r.status === filter) &&
      `${r.name} ${r.externalPort} ${r.internalPort} ${r.destinationIp} ${r.protocol}`
        .toLowerCase()
        .includes(search.toLowerCase()),
  );
  const title =
    page === 'ports'
      ? 'Port forwarding'
      : page === 'diagnostics'
        ? 'Network diagnostics'
        : page === 'profiles'
          ? 'Saved profiles'
          : 'A little help';

  async function runDiagnostics() {
    setDiagnosing(true);
    setError(null);
    try {
      setDiagnostics(await api.diagnose(includePublic));
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setDiagnosing(false);
    }
  }
  async function deleteProfile(id: string) {
    try {
      await api.deleteProfile(id);
      complete('Profile removed. Router rules were not changed.');
    } catch (e) {
      setError(errorMessage(e));
    }
  }
  async function copyCommand() {
    try {
      await navigator.clipboard.writeText('epf open 25565');
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2000);
    } catch {
      setToast('Select and copy the command: epf open 25565');
    }
  }

  return (
    <div className="app-shell" ref={motionScope}>
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <PlugsConnected size={27} weight="bold" />
          </div>
          <span>
            epf<span className="brand-period">.</span>
          </span>
          <span className="version">v0.1</span>
        </div>
        <p className="brand-description">Easy Port Forwarding</p>
        <nav aria-label="Main navigation">
          {nav.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              className={`nav-item ${page === id ? 'selected' : ''}`}
              onClick={() => setPage(id)}
              aria-current={page === id ? 'page' : undefined}
            >
              <Icon size={19} />
              {label}
              {id === 'ports' && active > 0 && <span className="nav-count">{active}</span>}
            </button>
          ))}
        </nav>
        <div className="sidebar-tip">
          <div className="tip-icon">
            <ShieldCheck size={22} />
          </div>
          <strong>You’re in control.</strong>
          <p>
            Only the ports you choose.
            <br />
            Only when you say so.
          </p>
          <button onClick={() => setPage('guide')}>
            How EPF works <ArrowUpRight size={14} />
          </button>
        </div>
        <div className="sidebar-bottom">
          <button
            className={`nav-item ${page === 'guide' ? 'selected' : ''}`}
            onClick={() => setPage('guide')}
          >
            <Question size={19} />
            Help & getting started
          </button>
          <div className="open-source">
            <span className="tiny-dot" />
            Free & open source<span>MIT</span>
          </div>
        </div>
      </aside>
      <div className="main-shell">
        <header className="topbar">
          <div className="breadcrumb">
            Workspace <span>/</span>
            <strong>{title}</strong>
          </div>
          <div className="topbar-right">
            {isPreview ? (
              <span className="preview-badge">
                <Circle size={9} weight="fill" />
                Browser preview
              </span>
            ) : (
              <span className="device-label">
                <Desktop size={15} />
                This computer
              </span>
            )}
            <span className="topbar-divider" />
            <span className="app-label">
              EPF <span>0.1.0</span>
            </span>
          </div>
        </header>
        <main>
          {isPreview && (
            <div className="preview-banner">
              <Info size={16} />
              <span>You’re exploring a sample network. No real router settings will change.</span>
            </div>
          )}
          <div className="page-heading">
            <div>
              <h1>{page === 'ports' ? 'Open a port. Get connected.' : title}</h1>
              <p>
                {page === 'ports'
                  ? 'Your connections, without the trip through router settings.'
                  : page === 'diagnostics'
                    ? 'Find out what’s working and what might be in the way.'
                    : page === 'profiles'
                      ? 'Save the setup once. Open it again when you need it.'
                      : 'A few things to know before opening your first port.'}
              </p>
            </div>
            {page === 'ports' && (
              <button className="button primary" onClick={() => setDialog({})} disabled={loading}>
                <Plus size={17} weight="bold" />
                Open a port
              </button>
            )}
            {page === 'profiles' && (
              <button className="button primary" onClick={() => setDialog({})}>
                <Plus size={17} />
                New profile
              </button>
            )}
          </div>
          {error && (
            <div className="error-banner" role="alert">
              <WarningCircle size={20} />
              <div>
                <strong>EPF couldn’t complete that.</strong>
                <p>{error}</p>
              </div>
              <button
                className="icon-button"
                onClick={() => setError(null)}
                aria-label="Dismiss error"
              >
                <X size={18} />
              </button>
            </div>
          )}

          {page === 'ports' && (
            <>
              <section className="network-panel" aria-labelledby="network-title">
                <div className="panel-heading">
                  <div className="network-label">
                    <span className="network-symbol">
                      <WifiHigh size={22} />
                    </span>
                    <div>
                      <h2 id="network-title">Your network</h2>
                      <span>
                        {network?.interface ??
                          (loading ? 'Finding your connection…' : 'No connection detected')}
                      </span>
                    </div>
                  </div>
                  <span className={`status-pill ${network?.upnpAvailable ? 'green' : 'neutral'}`}>
                    <span className="status-dot" />
                    {loading
                      ? 'Checking…'
                      : network?.upnpAvailable
                        ? 'Router connected'
                        : 'UPnP unavailable'}
                  </span>
                </div>
                <div className="network-stats">
                  <div>
                    <span className="stat-label">THIS COMPUTER</span>
                    <strong className="mono">{network?.localIp ?? '—'}</strong>
                    <small>Your local IP address</small>
                  </div>
                  <div>
                    <span className="stat-label">DEFAULT GATEWAY</span>
                    <strong className="mono">{network?.gatewayIp ?? '—'}</strong>
                    <small>{network?.routerName ?? 'Your router'}</small>
                  </div>
                  <div>
                    <span className="stat-label">ROUTER WAN IP</span>
                    <strong className="mono">{network?.wanIp ?? '—'}</strong>
                    <small>
                      {network?.wanIp ? 'Reported by your router' : 'Not available yet'}
                    </small>
                  </div>
                </div>
                <div className="network-footer">
                  <span>
                    <ShieldCheck size={16} />
                    {network?.upnpAvailable
                      ? 'UPnP available · Changes always need your confirmation'
                      : loading
                        ? 'Discovering your router. No rules are being changed.'
                        : 'Connect to a UPnP-enabled router to open ports.'}
                  </span>
                  <button className="inline-link" onClick={() => setPage('diagnostics')}>
                    View diagnostics <ArrowRight size={14} />
                  </button>
                </div>
              </section>
              {snapshot?.warning && (
                <div className="warning-banner">
                  <WarningCircle size={19} />
                  <p>{snapshot.warning}</p>
                  <button className="inline-link" onClick={() => setPage('guide')}>
                    Get help <ArrowRight size={14} />
                  </button>
                </div>
              )}
              <section className="rules-section" aria-labelledby="rules-title">
                <div className="section-heading">
                  <div className="section-title">
                    <h2 id="rules-title">Forwarding rules</h2>
                    <span className="count-badge">{rules.length}</span>
                  </div>
                  <button
                    className="button refresh-button"
                    onClick={() => void refresh()}
                    disabled={loading}
                  >
                    <ArrowsClockwise className={loading ? 'spinning' : ''} size={16} />
                    {loading ? 'Refreshing…' : 'Refresh rules'}
                  </button>
                </div>
                <div className="rules-panel">
                  <div className="table-toolbar">
                    <div className="segmented filter-tabs" aria-label="Filter forwarding rules">
                      {(['all', 'active', 'inactive'] as const).map((f) => (
                        <button key={f} aria-pressed={filter === f} onClick={() => setFilter(f)}>
                          {f === 'all' ? 'All rules' : f === 'active' ? 'Active' : 'Inactive'}
                          {f === 'all' && <span>{rules.length}</span>}
                        </button>
                      ))}
                    </div>
                    <div className="search-field">
                      <MagnifyingGlass size={17} />
                      <input
                        aria-label="Search rules"
                        placeholder="Find a rule…"
                        value={search}
                        onChange={(e) => setSearch(e.target.value)}
                      />
                    </div>
                  </div>
                  <div className="table-scroll">
                    <table>
                      <thead>
                        <tr>
                          <th>RULE / APPLICATION</th>
                          <th>PORT</th>
                          <th>PROTOCOL</th>
                          <th>DESTINATION</th>
                          <th>STATUS</th>
                          <th>
                            <span className="sr-only">Actions</span>
                          </th>
                        </tr>
                      </thead>
                      <tbody>
                        {filtered.map((rule) => (
                          <tr key={rule.id}>
                            <td>
                              <div className="rule-name">
                                <span
                                  className={`rule-app-icon ${rule.status === 'active' ? 'active-icon' : ''}`}
                                >
                                  <PlugsConnected size={20} />
                                </span>
                                <div>
                                  <strong>{rule.name}</strong>
                                  <small>
                                    {rule.leaseSeconds / 3600 < 1
                                      ? `${rule.leaseSeconds / 60} minute`
                                      : `${rule.leaseSeconds / 3600} hour`}{' '}
                                    lease
                                  </small>
                                </div>
                              </div>
                            </td>
                            <td>
                              <strong className="mono port-number">{rule.externalPort}</strong>
                              {rule.externalPort !== rule.internalPort && (
                                <small className="port-translation">→ {rule.internalPort}</small>
                              )}
                            </td>
                            <td>
                              <span className="protocol-badge">{rule.protocol.toUpperCase()}</span>
                            </td>
                            <td>
                              <span className="mono destination">{rule.destinationIp}</span>
                              <small className="destination-label">
                                {rule.destinationIp === network?.localIp
                                  ? 'This computer'
                                  : 'Local device'}
                              </small>
                            </td>
                            <td>
                              <span
                                className={`status-pill ${rule.status === 'active' ? 'green' : rule.status === 'conflict' || rule.status === 'pending' ? 'amber' : 'neutral'}`}
                              >
                                <span className="status-dot" />
                                {rule.status[0].toUpperCase() + rule.status.slice(1)}
                              </span>
                            </td>
                            <td>
                              {rule.status === 'active' ||
                              rule.status === 'pending' ||
                              rule.status === 'unknown' ? (
                                <button
                                  className="row-action"
                                  disabled={!network?.upnpAvailable || loading}
                                  onClick={() => setClosing(rule)}
                                >
                                  Close<span className="sr-only"> {rule.name} port</span>
                                </button>
                              ) : rule.status === 'inactive' ? (
                                <button
                                  className="row-action"
                                  disabled={!network?.upnpAvailable || loading}
                                  onClick={() => setDialog({ initial: rule })}
                                >
                                  Reopen<span className="sr-only"> {rule.name} port</span>
                                </button>
                              ) : (
                                <span className="muted">Review router</span>
                              )}
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                  {filtered.length === 0 && (
                    <div className="empty-state">
                      <span className="empty-icon">
                        <PlugsConnected size={28} />
                      </span>
                      <h3>
                        {search || filter !== 'all'
                          ? 'No matching rules'
                          : 'Your first connection starts here.'}
                      </h3>
                      <p>
                        {search || filter !== 'all'
                          ? 'Try a different search or show all rules.'
                          : 'Open a port for your game server, app, or anything you’re building.'}
                      </p>
                      {!search && filter === 'all' && (
                        <button className="button secondary" onClick={() => setDialog({})}>
                          <Plus size={16} />
                          Open your first port
                        </button>
                      )}
                    </div>
                  )}
                  <div className="table-footer">
                    <span>
                      <span className="tiny-dot" />
                      {active} active {active === 1 ? 'mapping' : 'mappings'}
                    </span>
                    <span>
                      {network ? `Last checked ${date(network.checkedAt)}` : 'Waiting for network'}
                    </span>
                  </div>
                </div>
              </section>
              <div className="below-table">
                <div className="reachability-note">
                  <Info size={18} />
                  <p>
                    <strong>A mapped port is a good start.</strong> Internet access also depends on
                    your firewall, ISP, and a running server.{' '}
                    <button className="inline-link" onClick={() => setPage('diagnostics')}>
                      Check your network <ArrowUpRight size={12} />
                    </button>
                  </p>
                </div>
                <div className="cli-teaser">
                  <TerminalWindow size={20} />
                  <div>
                    <strong>More of a terminal person?</strong>
                    <code>epf open 25565</code>
                  </div>
                  <button
                    className="icon-button"
                    aria-label="Copy CLI command"
                    onClick={() => void copyCommand()}
                  >
                    {copied ? <Check size={16} /> : <Copy size={16} />}
                  </button>
                </div>
              </div>
            </>
          )}

          {page === 'diagnostics' && (
            <>
              <section className="diagnostics-hero">
                <div className="network-route">
                  <div>
                    <span>
                      <Desktop size={30} />
                    </span>
                    <strong>This computer</strong>
                    <code>{network?.localIp ?? 'Unknown'}</code>
                  </div>
                  <div className="route-line">
                    <span />
                  </div>
                  <div>
                    <span>
                      <Router size={30} />
                    </span>
                    <strong>{network?.routerName ?? 'Your router'}</strong>
                    <code>{network?.gatewayIp ?? 'Unknown'}</code>
                  </div>
                  <div className="route-line unverified">
                    <span />
                  </div>
                  <div>
                    <span>
                      <GlobeHemisphereWest size={30} />
                    </span>
                    <strong>The internet</strong>
                    <code>Not verified</code>
                  </div>
                </div>
                <div className="diagnostic-controls">
                  <div>
                    <h2>Let’s check the connection.</h2>
                    <p>Read your network details and look for common NAT restrictions.</p>
                  </div>
                  <button
                    className="button primary"
                    onClick={() => void runDiagnostics()}
                    disabled={diagnosing || loading}
                  >
                    <ArrowsClockwise className={diagnosing ? 'spinning' : ''} size={16} />
                    {diagnosing ? 'Checking network…' : 'Run diagnostics'}
                  </button>
                </div>
                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    checked={includePublic}
                    onChange={(e) => setIncludePublic(e.target.checked)}
                  />
                  <span>
                    Include a public IP lookup{' '}
                    <small>
                      Sends an HTTPS request to api4.ipify.org
                      {isPreview ? ' in the desktop app. Preview uses sample data.' : '.'}
                    </small>
                  </span>
                </label>
              </section>
              {diagnostics ? (
                <section className="diagnostics-results" aria-live="polite">
                  <div className="section-heading">
                    <h2>Here’s what we found</h2>
                    <span className="muted">Read-only check</span>
                  </div>
                  <div className="diagnostic-addresses">
                    <div>
                      <span>Router WAN IP</span>
                      <strong className="mono">{diagnostics.network.wanIp ?? 'Unavailable'}</strong>
                    </div>
                    <div>
                      <span>Public IP {isPreview && '(sample)'}</span>
                      <strong className="mono">{diagnostics.publicIp ?? 'Not requested'}</strong>
                    </div>
                    <div>
                      <span>UPnP support</span>
                      <strong>
                        {diagnostics.network.upnpAvailable ? 'Available' : 'Unavailable'}
                      </strong>
                    </div>
                  </div>
                  {diagnostics.findings.map((finding, i) => (
                    <div className="finding" key={i}>
                      <Info size={20} />
                      <p>{finding}</p>
                    </div>
                  ))}
                  <div className="notice">
                    <ShieldCheck size={21} />
                    <p>{diagnostics.reachability}</p>
                  </div>
                </section>
              ) : (
                <div className="diagnostics-explainer">
                  <Info size={20} />
                  <p>
                    These checks won’t open or close any ports. CGNAT detection is an indication
                    based on your router’s WAN address; your ISP can confirm whether your connection
                    uses it.
                  </p>
                </div>
              )}
            </>
          )}

          {page === 'profiles' && (
            <section className="profiles-panel">
              {(snapshot?.profiles ?? []).length === 0 ? (
                <div className="empty-state spacious">
                  <span className="empty-icon">
                    <BookmarkSimple size={30} />
                  </span>
                  <h2>Keep your favorite setups handy.</h2>
                  <p>
                    Save a name, port, and protocol as a profile.
                    <br />
                    You’ll review it again before anything opens.
                  </p>
                  <button className="button primary" onClick={() => setDialog({})}>
                    <Plus size={16} />
                    Create a profile
                  </button>
                </div>
              ) : (
                <div className="profile-list">
                  {snapshot?.profiles.map((profile) => (
                    <div className="profile-row" key={profile.id}>
                      <span className="rule-app-icon">
                        <BookmarkSimple size={22} />
                      </span>
                      <div className="profile-info">
                        <h3>{profile.request.name}</h3>
                        <p>
                          <span className="mono">{profile.request.externalPort}</span> ·{' '}
                          {protocolLabel(profile.request.protocol)} ·{' '}
                          {profile.request.leaseSeconds / 60} minutes
                        </p>
                      </div>
                      <button
                        className="button secondary"
                        onClick={() => setDialog({ initial: profile.request })}
                      >
                        Use profile <ArrowUpRight size={15} />
                      </button>
                      <button
                        className="icon-button"
                        aria-label={`Delete ${profile.request.name} profile`}
                        onClick={() => void deleteProfile(profile.id)}
                      >
                        <Trash size={17} />
                      </button>
                    </div>
                  ))}
                </div>
              )}
              <div className="profile-footnote">
                <ShieldCheck size={16} />
                Saving a profile never changes your router.
              </div>
            </section>
          )}

          {page === 'guide' && (
            <div className="guide-layout">
              <section className="guide-main">
                <h2>Your first port, in three steps.</h2>
                {[
                  {
                    icon: WifiHigh,
                    title: 'Get on your home network.',
                    text: 'Connect over Wi-Fi or Ethernet. EPF finds your default gateway and checks whether it supports UPnP. A VPN or guest network can get in the way.',
                  },
                  {
                    icon: PlugsConnected,
                    title: 'Tell EPF what you need.',
                    text: 'Enter the port from your app or game’s instructions. Choose TCP, UDP, or both. Advanced options let you use a different internal port or another device on your local subnet.',
                  },
                  {
                    icon: ShieldCheck,
                    title: 'Review it, then open it.',
                    text: 'Check the destination and duration. EPF asks before changing your router. Keep the app or server listening on that port and allow it through your computer’s firewall.',
                  },
                ].map((step) => (
                  <div className="guide-step" key={step.title}>
                    <span>
                      <step.icon size={21} />
                    </span>
                    <div>
                      <h3>{step.title}</h3>
                      <p>{step.text}</p>
                    </div>
                  </div>
                ))}
                <button
                  className="button primary"
                  onClick={() => {
                    setPage('ports');
                    setDialog({});
                  }}
                >
                  Open your first port <ArrowRight size={16} />
                </button>
              </section>
              <aside className="guide-aside">
                <ShieldCheck size={26} />
                <h3>A few useful limits.</h3>
                <p>
                  EPF can’t bypass restrictions from your ISP. CGNAT or a second router may need an
                  upstream change or a tunnel.
                </p>
                <p>
                  Temporary leases expire on the router. Closing EPF doesn’t close them early, and
                  EPF never silently renews them.
                </p>
                <p>
                  Some routers don’t support UPnP or don’t let EPF inspect rules. They may need
                  manual configuration.
                </p>
                <button className="inline-link" onClick={() => setPage('diagnostics')}>
                  Check your network <ArrowRight size={15} />
                </button>
              </aside>
              <section className="cli-guide">
                <TerminalWindow size={23} />
                <div>
                  <h3>The same engine, in your terminal.</h3>
                  <pre>
                    <code>
                      {
                        'epf open 25565\nepf open 2456 --protocol udp\nepf list --refresh\nepf close 25565\nepf status\nepf diagnose --public-ip'
                      }
                    </code>
                  </pre>
                  <p>
                    The CLI asks for confirmation. Use <code>--yes</code> to explicitly approve a
                    scripted change. Download it separately from GitHub Releases.
                  </p>
                </div>
              </section>
            </div>
          )}
          <footer className="main-footer">
            <span>
              <ShieldCheck size={14} />
              Your network stays in your hands.
            </span>
          </footer>
        </main>
      </div>
      {dialog && (
        <RuleDialog
          open
          initial={dialog.initial}
          localIp={network?.localIp ?? null}
          canOpen={Boolean(network?.upnpAvailable)}
          onOpenChange={(value) => {
            if (!value) setDialog(null);
          }}
          onComplete={complete}
        />
      )}
      {closing && (
        <CloseDialog rule={closing} onClose={() => setClosing(null)} onComplete={complete} />
      )}
      {toast && (
        <div className="toast" role="status">
          <CheckCircle size={21} weight="fill" />
          <span>{toast}</span>
          <button
            className="icon-button"
            onClick={() => setToast(null)}
            aria-label="Dismiss notification"
          >
            <X size={16} />
          </button>
        </div>
      )}
    </div>
  );
}

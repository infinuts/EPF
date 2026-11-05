import { useState } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import {
  ArrowRight,
  CaretDown,
  Check,
  FloppyDisk,
  GlobeHemisphereWest,
  ShieldCheck,
  X,
} from '@phosphor-icons/react';
import { api, isPreview } from '../api';
import type { OpenRequest, Protocol } from '../types';
import { parsePort, validateRequest } from '../validation';

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initial?: OpenRequest;
  localIp: string | null;
  canOpen: boolean;
  onComplete: (message: string) => void;
}
const protocolText = (p: Protocol) => (p === 'both' ? 'TCP + UDP' : p.toUpperCase());

export default function RuleDialog({
  open,
  onOpenChange,
  initial,
  localIp,
  canOpen,
  onComplete,
}: Props) {
  const [name, setName] = useState(initial?.name ?? '');
  const [port, setPort] = useState(initial ? String(initial.externalPort) : '');
  const [internal, setInternal] = useState(
    initial && initial.internalPort !== initial.externalPort ? String(initial.internalPort) : '',
  );
  const [destination, setDestination] = useState(initial?.destinationIp ?? '');
  const [protocol, setProtocol] = useState<Protocol>(initial?.protocol ?? 'tcp');
  const [lease, setLease] = useState(initial?.leaseSeconds ?? 3600);
  const [advanced, setAdvanced] = useState(
    Boolean(initial?.destinationIp || (initial && initial.internalPort !== initial.externalPort)),
  );
  const [review, setReview] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const request = (): OpenRequest => ({
    name: name.trim() || `Port ${port}`,
    externalPort: parsePort(port) ?? 0,
    internalPort: parsePort(internal || port) ?? 0,
    destinationIp: destination.trim() || null,
    protocol,
    leaseSeconds: lease,
  });
  function validate() {
    const result = validateRequest(request());
    setError(result);
    return !result;
  }
  async function submit(saveOnly: boolean) {
    if (!validate()) return;
    setBusy(true);
    setError(null);
    try {
      if (saveOnly) await api.saveProfile(request());
      else await api.open(request(), true);
      onComplete(
        saveOnly
          ? 'Profile saved. Use it whenever you need it.'
          : isPreview
            ? 'Preview rule added. No router changes were made.'
            : 'Your router confirmed the mapping. Internet reachability is unverified.',
      );
      onOpenChange(false);
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <Dialog.Root
      open={open}
      onOpenChange={(value) => {
        if (!busy) onOpenChange(value);
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="dialog-overlay" />
        <Dialog.Content
          className="dialog-content"
          onInteractOutside={(e) => {
            if (busy) e.preventDefault();
          }}
        >
          <div className="dialog-heading">
            <span className="dialog-icon">
              <GlobeHemisphereWest size={24} />
            </span>
            <Dialog.Close className="icon-button" aria-label="Close dialog" disabled={busy}>
              <X size={20} />
            </Dialog.Close>
          </div>
          <Dialog.Title>
            {review ? 'Ready to open this port?' : 'Give your connection a way in.'}
          </Dialog.Title>
          <Dialog.Description>
            {review
              ? 'Review the details before EPF changes your router.'
              : 'Tell us which port you need. We’ll take care of the router.'}
          </Dialog.Description>
          {isPreview && (
            <div className="preview-notice">
              Browser preview. This flow only changes sample data.
            </div>
          )}
          {review ? (
            <>
              <div className="review-port">
                <span>{request().externalPort}</span>
                <span className="protocol-badge">{protocolText(protocol)}</span>
                <ArrowRight size={22} />
                <div>
                  <strong className="mono">{destination || localIp || 'This computer'}</strong>
                  <small>Internal port {request().internalPort}</small>
                </div>
              </div>
              <dl className="review-details">
                <div>
                  <dt>Rule name</dt>
                  <dd>{request().name}</dd>
                </div>
                <div>
                  <dt>Duration</dt>
                  <dd>
                    {lease / 3600 < 1
                      ? `${lease / 60} minutes`
                      : `${lease / 3600} hour${lease > 3600 ? 's' : ''}`}
                  </dd>
                </div>
                <div>
                  <dt>Internet accessibility</dt>
                  <dd>Not verified</dd>
                </div>
              </dl>
              <div className="notice">
                <ShieldCheck size={20} />
                <p>
                  This allows incoming connections to the selected service. Only open ports for apps
                  you trust. The rule will expire unless you explicitly renew it.
                </p>
              </div>
            </>
          ) : (
            <form
              id="port-form"
              onSubmit={(e) => {
                e.preventDefault();
                if (validate()) setReview(true);
              }}
            >
              <div className="preset-row">
                <span>Quick start</span>
                <button
                  type="button"
                  onClick={() => {
                    setName('Minecraft server');
                    setPort('25565');
                    setProtocol('tcp');
                  }}
                >
                  Minecraft <ArrowRight size={12} />
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setName('Valheim');
                    setPort('2456');
                    setProtocol('udp');
                  }}
                >
                  Valheim <ArrowRight size={12} />
                </button>
              </div>
              <label htmlFor="rule-name">
                Rule name <span className="optional">optional</span>
              </label>
              <input
                id="rule-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g. My game server"
                maxLength={64}
                autoFocus
              />
              <div className="form-grid">
                <div>
                  <label htmlFor="external-port">Port number</label>
                  <input
                    id="external-port"
                    className="port-input mono"
                    value={port}
                    onChange={(e) => setPort(e.target.value)}
                    placeholder="25565"
                    inputMode="numeric"
                    maxLength={5}
                    required
                  />
                </div>
                <fieldset>
                  <legend>Protocol</legend>
                  <div className="segmented protocol-selector">
                    {(['tcp', 'udp', 'both'] as const).map((p) => (
                      <button
                        type="button"
                        key={p}
                        aria-pressed={protocol === p}
                        onClick={() => setProtocol(p)}
                      >
                        {p === 'both' ? 'Both' : p.toUpperCase()}
                      </button>
                    ))}
                  </div>
                </fieldset>
              </div>
              <p className="field-help">
                Not sure about the protocol? Check the app or game’s instructions.
              </p>
              <button
                className="advanced-toggle"
                type="button"
                aria-expanded={advanced}
                aria-controls="advanced-fields"
                onClick={() => setAdvanced(!advanced)}
              >
                Advanced options <CaretDown className={advanced ? 'rotated' : ''} size={15} />
              </button>
              {advanced && (
                <div id="advanced-fields" className="advanced-fields">
                  <div className="form-grid">
                    <div>
                      <label htmlFor="internal-port">Internal port</label>
                      <input
                        id="internal-port"
                        value={internal}
                        onChange={(e) => setInternal(e.target.value)}
                        placeholder={port || 'Same as external'}
                        inputMode="numeric"
                        maxLength={5}
                      />
                    </div>
                    <div>
                      <label htmlFor="destination">Destination IPv4</label>
                      <input
                        id="destination"
                        value={destination}
                        onChange={(e) => setDestination(e.target.value)}
                        placeholder={localIp ?? 'This computer'}
                      />
                    </div>
                  </div>
                  <label htmlFor="lease">Keep open for</label>
                  <select
                    id="lease"
                    value={lease}
                    onChange={(e) => setLease(Number(e.target.value))}
                  >
                    <option value={900}>15 minutes</option>
                    <option value={3600}>1 hour (recommended)</option>
                    <option value={14400}>4 hours</option>
                    <option value={86400}>24 hours</option>
                  </select>
                  <p className="field-help">
                    Temporary by design. EPF never renews a rule without your confirmation.
                  </p>
                </div>
              )}
            </form>
          )}
          {error && (
            <p className="form-error" role="alert">
              {error}
            </p>
          )}
          {!canOpen && (
            <p className="form-error">
              No supported router is connected. You can still save this configuration as a profile.
            </p>
          )}
          <div className="dialog-footer">
            {review ? (
              <>
                <button
                  className="button secondary"
                  onClick={() => setReview(false)}
                  disabled={busy}
                >
                  Back
                </button>
                <button
                  className="button primary"
                  disabled={busy || !canOpen}
                  onClick={() => void submit(false)}
                >
                  <Check size={16} />
                  {busy ? 'Opening port…' : 'Open port'}
                </button>
              </>
            ) : (
              <>
                <button
                  className="button text-button"
                  disabled={busy}
                  onClick={() => void submit(true)}
                >
                  <FloppyDisk size={17} />
                  Save as profile
                </button>
                <button
                  type="submit"
                  form="port-form"
                  className="button primary"
                  disabled={busy || !canOpen}
                >
                  Review rule <ArrowRight size={16} />
                </button>
              </>
            )}
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

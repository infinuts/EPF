import { useState } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { ShieldCheck, X } from '@phosphor-icons/react';
import { api, isPreview } from '../api';
import type { Rule } from '../types';

export default function CloseDialog({
  rule,
  onClose,
  onComplete,
}: {
  rule: Rule;
  onClose: () => void;
  onComplete: (message: string) => void;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function close() {
    setBusy(true);
    try {
      await api.close(rule.id, true);
      onComplete(
        isPreview ? 'Preview rule closed. No router changes were made.' : 'Router mapping closed.',
      );
      onClose();
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <Dialog.Root
      open
      onOpenChange={(open) => {
        if (!open && !busy) onClose();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="dialog-overlay" />
        <Dialog.Content className="dialog-content small-dialog">
          <div className="dialog-heading">
            <span className="dialog-icon">
              <ShieldCheck size={24} />
            </span>
            <Dialog.Close className="icon-button" aria-label="Close dialog" disabled={busy}>
              <X size={20} />
            </Dialog.Close>
          </div>
          <Dialog.Title>Close this port?</Dialog.Title>
          <Dialog.Description>
            EPF will remove its {rule.protocol.toUpperCase()} mapping for port {rule.externalPort}.{' '}
            {rule.name} will stop receiving connections through this rule.
          </Dialog.Description>
          {isPreview && <div className="preview-notice">This only changes sample data.</div>}
          <p className="field-help">
            The saved configuration stays in your rule history. You can reopen it later.
          </p>
          {error && (
            <p role="alert" className="form-error">
              {error}
            </p>
          )}
          <div className="dialog-footer">
            <Dialog.Close className="button secondary" disabled={busy}>
              Keep open
            </Dialog.Close>
            <button className="button danger" onClick={() => void close()} disabled={busy}>
              {busy ? 'Closing…' : 'Close port'}
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

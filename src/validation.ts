import type { OpenRequest } from './types';

export function parsePort(value: string): number | null {
  if (!/^\d{1,5}$/.test(value)) return null;
  const port = Number(value);
  return port >= 1 && port <= 65535 ? port : null;
}

export function isIPv4(value: string): boolean {
  const parts = value.split('.');
  if (parts.length !== 4 || !parts.every((p) => /^(0|[1-9]\d{0,2})$/.test(p) && Number(p) <= 255))
    return false;
  const [a, b] = parts.map(Number);
  return a > 0 && a < 224 && a !== 127 && !(a === 169 && b === 254);
}

export function validateRequest(request: OpenRequest): string | null {
  if (!parsePort(String(request.externalPort)) || !parsePort(String(request.internalPort)))
    return 'Enter a port between 1 and 65535.';
  if (
    !request.name.trim() ||
    new TextEncoder().encode(request.name).length > 64 ||
    [...request.name].some((c) => c.charCodeAt(0) < 32 || c.charCodeAt(0) === 127)
  )
    return 'Use a rule name of 1 to 64 characters without control characters.';
  if (request.destinationIp && !isIPv4(request.destinationIp))
    return 'Enter a valid unicast IPv4 destination.';
  if (request.leaseSeconds < 60 || request.leaseSeconds > 86400)
    return 'Choose a lease between 60 seconds and 24 hours.';
  return null;
}

import { describe, it, expect } from 'vitest';
import { isIPv4, parsePort, validateRequest } from './validation';

describe('port input validation', () => {
  it('accepts integer ports only', () => {
    for (const input of ['0', '-1', '65536', '3.5', '3e3', '123abc', '', ' 80'])
      expect(parsePort(input)).toBeNull();
    expect(parsePort('1')).toBe(1);
    expect(parsePort('65535')).toBe(65535);
  });
  it('rejects invalid destinations', () => {
    for (const input of [
      '127.0.0.1',
      '256.1.2.3',
      '1.2.3',
      '0.0.0.0',
      '255.255.255.255',
      '224.0.0.1',
      '169.254.1.1',
      '01.2.3.4',
    ])
      expect(isIPv4(input)).toBe(false);
    expect(isIPv4('192.168.1.24')).toBe(true);
  });
  it('validates names in UTF-8 bytes, ports, and finite leases', () => {
    const request = {
      name: 'Minecraft',
      externalPort: 25565,
      internalPort: 25565,
      protocol: 'tcp' as const,
      destinationIp: null,
      leaseSeconds: 3600,
    };
    expect(validateRequest(request)).toBeNull();
    expect(validateRequest({ ...request, name: 'é'.repeat(33) })).not.toBeNull();
    expect(validateRequest({ ...request, leaseSeconds: 0 })).not.toBeNull();
    expect(validateRequest({ ...request, name: 'bad\nname' })).not.toBeNull();
  });
});

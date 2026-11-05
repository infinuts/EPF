export type Protocol = 'tcp' | 'udp' | 'both';
export type RuleStatus = 'active' | 'inactive' | 'pending' | 'conflict' | 'unknown';
export interface OpenRequest {
  name: string;
  externalPort: number;
  internalPort: number;
  protocol: Protocol;
  destinationIp: string | null;
  leaseSeconds: number;
}
export interface Rule extends Omit<OpenRequest, 'destinationIp' | 'protocol'> {
  id: string;
  protocol: 'tcp' | 'udp';
  destinationIp: string;
  createdAt: number;
  routerId: string;
  description: string;
  status: RuleStatus;
}
export interface Profile {
  id: string;
  request: OpenRequest;
}
export interface NetworkStatus {
  localIp: string | null;
  gatewayIp: string | null;
  interface: string | null;
  routerName: string | null;
  manufacturer: string | null;
  model: string | null;
  routerId: string | null;
  wanIp: string | null;
  upnpAvailable: boolean;
  message: string;
  checkedAt: number;
}
export interface Snapshot {
  network: NetworkStatus;
  rules: Rule[];
  profiles: Profile[];
  warning: string | null;
}
export interface Diagnostics {
  network: NetworkStatus;
  publicIp: string | null;
  nat: 'unknown' | 'cgnat' | 'doublenat' | 'mismatch' | 'public';
  findings: string[];
  reachability: string;
}

/** Type-only Phase 1.5 contracts. Implementations belong to future phases. */
export type RiskLevel =
  'safe' | 'confirmation_required' | 'administrative_approval' | 'system_critical';
export type ErrorCategory =
  | 'SYSTEM'
  | 'RUNTIME'
  | 'AGENT'
  | 'PLUGIN'
  | 'TOOL'
  | 'PERMISSION'
  | 'MODEL'
  | 'NETWORK'
  | 'VALIDATION'
  | 'CONFLICT';

export interface ContractEnvelope {
  contract_version: string;
  correlation_id: string;
  causation_id?: string;
  timestamp: string;
  source: string;
  classification: 'public' | 'internal' | 'sensitive' | 'restricted';
}
export interface ContractError {
  code: string;
  category: ErrorCategory;
  retryable: boolean;
  message: string;
  details?: Record<string, unknown>;
  correlation_id: string;
}
export interface CapabilityDescriptor {
  capability_id: string;
  version: string;
  input_schema: string;
  output_schema: string;
  required_permissions: string[];
  failure_modes: string[];
  idempotency: 'required' | 'supported' | 'not_supported';
  rollback: 'supported' | 'unsupported' | 'compensating';
}
export interface ServiceDescriptor {
  service_id: string;
  instance_id: string;
  contract_versions: string[];
  capabilities: string[];
  dependencies: string[];
}
export interface ToolContract {
  descriptor: CapabilityDescriptor;
  initialize(): Promise<void>;
  health_check(): Promise<'ready' | 'degraded' | 'unhealthy'>;
  validate(input: unknown): Promise<void>;
  execute(input: unknown, envelope: ContractEnvelope): Promise<unknown>;
  rollback?(receipt: unknown): Promise<void>;
  shutdown(): Promise<void>;
}
export interface ModelProviderContract {
  discover_models(): Promise<unknown[]>;
  report_capabilities(): Promise<CapabilityDescriptor[]>;
  load(model_id: string): Promise<void>;
  unload(model_id: string): Promise<void>;
  infer(request: unknown, envelope: ContractEnvelope): AsyncIterable<unknown>;
  cancel(request_id: string): Promise<void>;
  health_check(): Promise<'ready' | 'degraded' | 'unavailable'>;
}

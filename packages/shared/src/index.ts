/** Shared, platform-neutral primitives. */
export type Result<T, E extends Error = Error> = { ok: true; value: T } | { ok: false; error: E };

export abstract class CognyxError extends Error {
  public abstract readonly code: string;
}

export class ConfigurationError extends CognyxError {
  readonly code = 'CONFIGURATION_ERROR';
}

export class PlatformNotSupportedError extends CognyxError {
  readonly code = 'PLATFORM_NOT_SUPPORTED';
}

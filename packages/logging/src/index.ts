export type LogLevel = 'DEBUG' | 'INFO' | 'WARNING' | 'ERROR' | 'CRITICAL';
export interface StructuredLogger {
  log(level: LogLevel, event: string, context?: Record<string, unknown>): void;
}
export class JsonLogger implements StructuredLogger {
  log(level: LogLevel, event: string, context: Record<string, unknown> = {}): void {
    console.log(JSON.stringify({ timestamp: new Date().toISOString(), level, event, ...context }));
  }
}

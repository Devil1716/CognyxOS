export type RuntimeEnvironment = 'development' | 'test' | 'production';
export interface AppConfig {
  environment: RuntimeEnvironment;
  logLevel: 'DEBUG' | 'INFO' | 'WARNING' | 'ERROR' | 'CRITICAL';
  pluginDirectory: string;
}
export function loadConfig(env: Record<string, string | undefined> = process.env): AppConfig {
  const environment = env.COGNYX_ENV ?? 'development';
  if (!['development', 'test', 'production'].includes(environment))
    throw new Error(`Invalid COGNYX_ENV: ${environment}`);
  const logLevel = env.COGNYX_LOG_LEVEL ?? 'INFO';
  if (!['DEBUG', 'INFO', 'WARNING', 'ERROR', 'CRITICAL'].includes(logLevel))
    throw new Error(`Invalid COGNYX_LOG_LEVEL: ${logLevel}`);
  return {
    environment: environment as RuntimeEnvironment,
    logLevel: logLevel as AppConfig['logLevel'],
    pluginDirectory: env.COGNYX_PLUGIN_DIR ?? './plugins',
  };
}

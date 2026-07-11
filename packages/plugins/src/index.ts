export interface PluginManifest {
  id: string;
  version: string;
  apiVersion: string;
  dependencies?: Record<string, string>;
}
export interface CognyxPlugin {
  manifest: PluginManifest;
  start(): Promise<void>;
  stop(): Promise<void>;
}
export class PluginRegistry {
  private readonly plugins = new Map<string, CognyxPlugin>();
  register(plugin: CognyxPlugin): void {
    if (plugin.manifest.apiVersion !== '1')
      throw new Error(`Incompatible plugin API: ${plugin.manifest.id}`);
    if (this.plugins.has(plugin.manifest.id))
      throw new Error(`Plugin already registered: ${plugin.manifest.id}`);
    this.plugins.set(plugin.manifest.id, plugin);
  }
  async load(id: string): Promise<void> {
    await this.require(id).start();
  }
  async unload(id: string): Promise<void> {
    await this.require(id).stop();
  }
  private require(id: string): CognyxPlugin {
    const plugin = this.plugins.get(id);
    if (!plugin) throw new Error(`Unknown plugin: ${id}`);
    return plugin;
  }
}

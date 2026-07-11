import type { PlatformServices, SystemInformation } from '../index.js';

/** Windows reference adapter. Native integrations will live behind this implementation. */
export class WindowsPlatformServices implements PlatformServices {
  async systemInformation(): Promise<SystemInformation> {
    return { platform: 'windows', hostname: 'unavailable-in-web-context' };
  }
  async copyToClipboard(text: string): Promise<void> {
    await navigator.clipboard.writeText(text);
  }
  async notify(title: string, body: string): Promise<void> {
    new Notification(title, { body });
  }
}

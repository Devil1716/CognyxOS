/**
 * Operating-system boundary. Application code depends only on these contracts.
 * Windows has the reference adapter; Linux and macOS adapters remain explicit future work.
 */
export interface SystemInformation {
  platform: 'windows' | 'linux' | 'macos';
  hostname: string;
}

export interface PlatformServices {
  systemInformation(): Promise<SystemInformation>;
  copyToClipboard(text: string): Promise<void>;
  notify(title: string, body: string): Promise<void>;
}

export class UnsupportedPlatformAdapter implements PlatformServices {
  async systemInformation(): Promise<SystemInformation> {
    throw new Error('Platform adapter is not implemented.');
  }
  async copyToClipboard(): Promise<void> {
    throw new Error('Platform adapter is not implemented.');
  }
  async notify(): Promise<void> {
    throw new Error('Platform adapter is not implemented.');
  }
}

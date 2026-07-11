import { describe, expect, it } from 'vitest';
import { ConfigurationError } from './index.js';

describe('shared errors', () => {
  it('provides a stable error code', () => {
    expect(new ConfigurationError('invalid').code).toBe('CONFIGURATION_ERROR');
  });
});

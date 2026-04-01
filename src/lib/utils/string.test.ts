import { describe, it, expect } from 'vitest';
import { generateSlug } from './string';

describe('generateSlug', () => {
  it('converts simple names', () => {
    expect(generateSlug('My Provider')).toBe('my-provider');
  });

  it('handles special characters', () => {
    expect(generateSlug('Hello, World!')).toBe('hello-world');
  });

  it('handles multiple spaces', () => {
    expect(generateSlug('  foo   bar  ')).toBe('foo-bar');
  });

  it('handles Chinese characters', () => {
    expect(generateSlug('阿里云')).toBe('');
  });

  it('handles empty string', () => {
    expect(generateSlug('')).toBe('');
  });

  it('handles already slug-like input', () => {
    expect(generateSlug('my-provider')).toBe('my-provider');
  });

  it('handles mixed alphanumeric', () => {
    expect(generateSlug('Provider 123')).toBe('provider-123');
  });
});

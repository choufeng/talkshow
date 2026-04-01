import { describe, it, expect } from 'vitest';
import { formatTime, formatTimestamp, formatDate } from './format';

describe('formatTime', () => {
  it('formats 0 seconds as 00:00', () => {
    expect(formatTime(0)).toBe('00:00');
  });

  it('formats seconds only', () => {
    expect(formatTime(5)).toBe('00:05');
    expect(formatTime(59)).toBe('00:59');
  });

  it('formats minutes and seconds', () => {
    expect(formatTime(60)).toBe('01:00');
    expect(formatTime(90)).toBe('01:30');
    expect(formatTime(3599)).toBe('59:59');
  });

  it('formats large values', () => {
    expect(formatTime(3600)).toBe('60:00');
    expect(formatTime(3661)).toBe('61:01');
  });
});

describe('formatTimestamp', () => {
  it('formats a valid ISO timestamp', () => {
    const result = formatTimestamp('2026-04-01T12:30:45.000Z');
    expect(result).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/);
  });

  it('returns NaN-formatted string for invalid input', () => {
    const result = formatTimestamp('not-a-date');
    expect(result).toContain('NaN');
  });

  it('returns NaN-formatted string for empty string', () => {
    const result = formatTimestamp('');
    expect(result).toContain('NaN');
  });
});

describe('formatDate', () => {
  it('formats a valid ISO date string', () => {
    const result = formatDate('2026-04-01T00:00:00.000Z');
    expect(result).toBeTruthy();
    expect(result.length).toBeGreaterThan(0);
  });

  it('returns Invalid Date for invalid input', () => {
    expect(formatDate('not-a-date')).toBe('Invalid Date');
  });

  it('returns Invalid Date for empty string', () => {
    expect(formatDate('')).toBe('Invalid Date');
  });
});

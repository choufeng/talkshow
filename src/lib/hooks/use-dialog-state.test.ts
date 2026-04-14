import { describe, it, expect, vi } from 'vitest';
import { createDialogState } from './use-dialog-state.svelte';

describe('createDialogState', () => {
  it('defaults to closed', () => {
    const state = createDialogState();
    expect(state.isOpen).toBe(false);
  });

  it('initializes with initialOpen=true', () => {
    const state = createDialogState({ initialOpen: true });
    expect(state.isOpen).toBe(true);
  });

  it('opens when open() called', () => {
    const state = createDialogState();
    state.open();
    expect(state.isOpen).toBe(true);
  });

  it('closes when close() called', () => {
    const state = createDialogState({ initialOpen: true });
    state.close();
    expect(state.isOpen).toBe(false);
  });

  it('onOpenChange(true) opens', () => {
    const state = createDialogState();
    state.onOpenChange(true);
    expect(state.isOpen).toBe(true);
  });

  it('onOpenChange(false) closes', () => {
    const state = createDialogState({ initialOpen: true });
    state.onOpenChange(false);
    expect(state.isOpen).toBe(false);
  });

  it('calls onReset when closing', () => {
    const onReset = vi.fn();
    const state = createDialogState({ initialOpen: true, onReset });
    state.close();
    expect(onReset).toHaveBeenCalledTimes(1);
  });

  it('calls setReset function when closing', () => {
    const resetFn = vi.fn();
    const state = createDialogState({ initialOpen: true });
    state.setReset(resetFn);
    state.close();
    expect(resetFn).toHaveBeenCalledTimes(1);
  });

  it('replaces reset function via setReset', () => {
    const firstReset = vi.fn();
    const secondReset = vi.fn();
    const state = createDialogState({ initialOpen: true, onReset: firstReset });
    state.setReset(secondReset);
    state.close();
    expect(firstReset).not.toHaveBeenCalled();
    expect(secondReset).toHaveBeenCalledTimes(1);
  });

  it('does not call reset on open', () => {
    const onReset = vi.fn();
    const state = createDialogState({ onReset });
    state.open();
    expect(onReset).not.toHaveBeenCalled();
  });

  it('re-opens when open() called while already open', () => {
    const state = createDialogState({ initialOpen: true });
    expect(state.isOpen).toBe(true);
    state.open();
    expect(state.isOpen).toBe(true);
  });

  it('toggles open-close-open when open() called twice', () => {
    const state = createDialogState();
    state.open();
    expect(state.isOpen).toBe(true);
    state.open();
    expect(state.isOpen).toBe(true);
  });
});

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import Toggle from './index.svelte';

describe('Toggle', () => {
  it('renders as switch role', () => {
    render(Toggle, { props: { checked: false } });
    const button = screen.getByRole('switch');
    expect(button).toBeInTheDocument();
  });

  it('reflects checked state via aria-checked', () => {
    render(Toggle, { props: { checked: true } });
    const button = screen.getByRole('switch');
    expect(button).toHaveAttribute('aria-checked', 'true');
  });

  it('reflects unchecked state via aria-checked', () => {
    render(Toggle, { props: { checked: false } });
    const button = screen.getByRole('switch');
    expect(button).toHaveAttribute('aria-checked', 'false');
  });

  it('calls onCheckedChange when clicked', async () => {
    const onChange = vi.fn();
    render(Toggle, { props: { checked: false, onCheckedChange: onChange } });
    const button = screen.getByRole('switch');
    await fireEvent.click(button);
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it('calls onCheckedChange with false when toggling from checked', async () => {
    const onChange = vi.fn();
    render(Toggle, { props: { checked: true, onCheckedChange: onChange } });
    const button = screen.getByRole('switch');
    await fireEvent.click(button);
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it('does not call onCheckedChange when disabled', async () => {
    const onChange = vi.fn();
    render(Toggle, { props: { checked: false, onCheckedChange: onChange, disabled: true } });
    const button = screen.getByRole('switch');
    expect(button).toBeDisabled();
    await fireEvent.click(button);
    expect(onChange).not.toHaveBeenCalled();
  });

  it('applies aria-label when provided', () => {
    render(Toggle, { props: { checked: false, ariaLabel: 'Enable feature' } });
    const button = screen.getByRole('switch');
    expect(button).toHaveAttribute('aria-label', 'Enable feature');
  });

  it('applies size classes', () => {
    const { container } = render(Toggle, { props: { checked: false, size: 'sm' } });
    const button = container.querySelector('button');
    expect(button?.className).toContain('h-5');
  });
});

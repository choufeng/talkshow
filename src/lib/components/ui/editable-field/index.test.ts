import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import EditableField from './index.svelte';

describe('EditableField', () => {
  it('displays masked value in password mode', () => {
    render(EditableField, { props: { value: 'sk-1234567890', mode: 'password', onChange: vi.fn() } });
    const display = screen.getByTitle('点击编辑');
    expect(display.textContent).toContain('sk-');
    expect(display.textContent).toContain('•');
  });

  it('displays plain value in text mode', () => {
    render(EditableField, { props: { value: 'hello world', mode: 'text', onChange: vi.fn() } });
    const display = screen.getByTitle('点击编辑');
    expect(display.textContent).toContain('hello world');
  });

  it('shows placeholder when value is empty', () => {
    render(EditableField, { props: { value: '', mode: 'text', onChange: vi.fn(), placeholder: 'Enter value' } });
    const display = screen.getByTitle('点击编辑');
    expect(display.textContent).toContain('Enter value');
  });

  it('enters edit mode on click', async () => {
    render(EditableField, { props: { value: 'test', mode: 'text', onChange: vi.fn() } });
    await fireEvent.click(screen.getByTitle('点击编辑'));
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  it('confirms edit on Enter key', async () => {
    const onChange = vi.fn();
    render(EditableField, { props: { value: 'old', mode: 'text', onChange } });
    await fireEvent.click(screen.getByTitle('点击编辑'));

    const input = screen.getByRole('textbox');
    await fireEvent.input(input, { target: { value: 'new value' } });
    await fireEvent.keyDown(input, { key: 'Enter' });

    expect(onChange).toHaveBeenCalledWith('new value');
  });

  it('cancels edit on Escape key', async () => {
    const onChange = vi.fn();
    render(EditableField, { props: { value: 'original', mode: 'text', onChange } });
    await fireEvent.click(screen.getByTitle('点击编辑'));

    const input = screen.getByRole('textbox');
    await fireEvent.input(input, { target: { value: 'changed' } });
    await fireEvent.keyDown(input, { key: 'Escape' });

    expect(onChange).not.toHaveBeenCalled();
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument();
  });

  it('confirms edit on confirm button click', async () => {
    const onChange = vi.fn();
    render(EditableField, { props: { value: 'old', mode: 'text', onChange } });
    await fireEvent.click(screen.getByTitle('点击编辑'));

    const input = screen.getByRole('textbox');
    await fireEvent.input(input, { target: { value: 'confirmed' } });

    const buttons = screen.getAllByRole('button');
    const confirmBtn = buttons.find(b => b.getAttribute('title') === '确认');
    expect(confirmBtn).toBeTruthy();
    await fireEvent.click(confirmBtn!);

    expect(onChange).toHaveBeenCalledWith('confirmed');
  });

  it('cancels edit on cancel button click', async () => {
    const onChange = vi.fn();
    render(EditableField, { props: { value: 'original', mode: 'text', onChange } });
    await fireEvent.click(screen.getByTitle('点击编辑'));

    const input = screen.getByRole('textbox');
    await fireEvent.input(input, { target: { value: 'changed' } });

    const buttons = screen.getAllByRole('button');
    const cancelBtn = buttons.find(b => b.getAttribute('title') === '取消');
    expect(cancelBtn).toBeTruthy();
    await fireEvent.click(cancelBtn!);

    expect(onChange).not.toHaveBeenCalled();
  });
});

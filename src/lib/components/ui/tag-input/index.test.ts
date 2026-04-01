import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import TagInput from './index.svelte';

describe('TagInput', () => {
  it('renders existing tags', () => {
    render(TagInput, { props: { tags: ['tag1', 'tag2'], onAdd: vi.fn(), onRemove: vi.fn() } });
    expect(screen.getByText('tag1')).toBeInTheDocument();
    expect(screen.getByText('tag2')).toBeInTheDocument();
  });

  it('calls onRemove when tag remove button clicked', async () => {
    const onRemove = vi.fn();
    render(TagInput, { props: { tags: ['tag1'], onAdd: vi.fn(), onRemove } });
    const removeBtn = screen.getByText('✕').closest('button')!;
    await fireEvent.click(removeBtn);
    expect(onRemove).toHaveBeenCalledWith('tag1');
  });

  it('shows input on add button click', async () => {
    render(TagInput, { props: { tags: [], onAdd: vi.fn(), onRemove: vi.fn() } });
    const addBtn = screen.getByText('+ 添加模型');
    await fireEvent.click(addBtn);
    expect(screen.getByPlaceholderText('添加...')).toBeInTheDocument();
  });

  it('calls onAdd when Enter pressed in input', async () => {
    const onAdd = vi.fn();
    render(TagInput, { props: { tags: [], onAdd, onRemove: vi.fn() } });
    await fireEvent.click(screen.getByText('+ 添加模型'));

    const input = screen.getByPlaceholderText('添加...');
    await fireEvent.input(input, { target: { value: 'new-tag' } });
    await fireEvent.keyDown(input, { key: 'Enter' });

    expect(onAdd).toHaveBeenCalledWith('new-tag');
  });

  it('does not add duplicate tag', async () => {
    const onAdd = vi.fn();
    render(TagInput, { props: { tags: ['existing'], onAdd, onRemove: vi.fn() } });
    await fireEvent.click(screen.getByText('+ 添加模型'));

    const input = screen.getByPlaceholderText('添加...');
    await fireEvent.input(input, { target: { value: 'existing' } });
    await fireEvent.keyDown(input, { key: 'Enter' });

    expect(onAdd).not.toHaveBeenCalled();
  });

  it('does not add empty tag', async () => {
    const onAdd = vi.fn();
    render(TagInput, { props: { tags: [], onAdd, onRemove: vi.fn() } });
    await fireEvent.click(screen.getByText('+ 添加模型'));

    const input = screen.getByPlaceholderText('添加...');
    await fireEvent.input(input, { target: { value: '   ' } });
    await fireEvent.keyDown(input, { key: 'Enter' });

    expect(onAdd).not.toHaveBeenCalled();
  });

  it('hides input on Escape', async () => {
    render(TagInput, { props: { tags: [], onAdd: vi.fn(), onRemove: vi.fn() } });
    await fireEvent.click(screen.getByText('+ 添加模型'));

    const input = screen.getByPlaceholderText('添加...');
    await fireEvent.keyDown(input, { key: 'Escape' });

    expect(screen.queryByPlaceholderText('添加...')).not.toBeInTheDocument();
  });
});

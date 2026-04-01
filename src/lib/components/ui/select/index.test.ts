import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import Select from './index.svelte';

describe('Select', () => {
  const groups = [
    {
      label: '内置',
      items: [
        { value: 'vertex', label: 'Vertex AI' },
        { value: 'dashscope', label: '阿里云' },
      ],
    },
    {
      label: '自定义',
      items: [
        { value: 'custom', label: 'Custom Provider' },
      ],
    },
  ];

  it('renders trigger with placeholder when no value matches', () => {
    render(Select, { props: { value: 'unknown', groups, placeholder: '请选择', onChange: () => {} } });
    expect(screen.getByText('请选择')).toBeInTheDocument();
  });

  it('renders trigger with selected value label', () => {
    render(Select, { props: { value: 'vertex', groups, onChange: () => {} } });
    expect(screen.getByText(/Vertex AI/)).toBeInTheDocument();
  });

  it('renders trigger with group prefix for selected item', () => {
    render(Select, { props: { value: 'vertex', groups, onChange: () => {} } });
    const trigger = screen.getByText(/内置.*Vertex AI/);
    expect(trigger).toBeInTheDocument();
  });
});

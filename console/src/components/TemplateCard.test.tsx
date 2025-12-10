import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { PolicyTemplate } from '@/types';
import { TemplateCard } from './TemplateCard';

const baseTemplate: PolicyTemplate = {
  id: 'database_read_only',
  name: 'Database Read Only',
  description: 'Allow agents to read data',
  template_text: 'Allow reading data',
  category: 'database',
  example_customizations: [
    'only analytics db',
    'exclude pii',
    'weekdays only',
  ],
};

describe('TemplateCard', () => {
  it('renders template meta and examples', () => {
    render(<TemplateCard template={baseTemplate} onSelect={() => {}} />);

    expect(screen.getByText('Database Read Only')).toBeInTheDocument();
    expect(
      screen.getByText('Allow agents to read data')
    ).toBeInTheDocument();
    expect(screen.getByText('only analytics db')).toBeInTheDocument();
    expect(screen.getByText('exclude pii')).toBeInTheDocument();
  });

  it('highlights when selected', () => {
    const { container } = render(
      <TemplateCard template={baseTemplate} selected onSelect={() => {}} />
    );

    expect(container.firstChild).toHaveClass('ring-2');
  });

  it('calls onSelect when clicked', async () => {
    const user = userEvent.setup();
    const handleSelect = vi.fn();

    render(<TemplateCard template={baseTemplate} onSelect={handleSelect} />);

    await user.click(screen.getByRole('button', { name: /database read only/i }));
    expect(handleSelect).toHaveBeenCalled();
  });
});

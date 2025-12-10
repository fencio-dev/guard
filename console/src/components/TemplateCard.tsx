import { Badge } from '@/components/ui/badge';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { cn } from '@/lib/utils';
import type { PolicyTemplate } from '@/types';
import { memo } from 'react';

interface TemplateCardProps {
  template: PolicyTemplate;
  selected?: boolean;
  onSelect: () => void;
}

export const TemplateCard = memo(function TemplateCard({
  template,
  selected = false,
  onSelect,
}: TemplateCardProps) {
  const examples = template.example_customizations?.slice(0, 3) ?? [];

  const handleKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onSelect();
    }
  };

  return (
    <Card
      role="button"
      tabIndex={0}
      aria-pressed={selected}
      aria-label={template.name}
      onClick={onSelect}
      onKeyDown={handleKeyDown}
      hoverable
      className={cn(
        'h-full cursor-pointer text-left transition-all hover:shadow-lg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-primary',
        selected && 'ring-2 ring-primary'
      )}
    >
      <CardHeader>
        <div className="flex items-start justify-between gap-3">
          <CardTitle className="text-lg font-semibold">{template.name}</CardTitle>
          <Badge variant="outline" className="uppercase tracking-wide">
            {template.category}
          </Badge>
        </div>
        <CardDescription>{template.description}</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-3">
          <div>
            <p className="text-sm font-medium text-muted-foreground">Template</p>
            <p className="text-sm italic text-muted-foreground">“{template.template_text}”</p>
          </div>

          {examples.length > 0 && (
            <div>
              <p className="text-sm font-medium text-muted-foreground">Example customizations</p>
              <ul className="list-disc list-inside text-sm text-muted-foreground/90">
                {examples.map((example, index) => (
                  <li key={`${template.id}-example-${index}`}>{example}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
});

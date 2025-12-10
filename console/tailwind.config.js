/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ['class'],
  content: [
    './index.html',
    './src/**/*.{js,ts,jsx,tsx}',
  ],
  theme: {
    container: {
      center: true,
      padding: '2rem',
      screens: {
        '2xl': '1400px',
      },
    },
    extend: {
      backgroundImage: {
        'gradient-hero': 'linear-gradient(to bottom, white, rgb(163 163 163))', /* For gradient text */
      },
      colors: {
        border: 'hsl(var(--border))',
        input: 'hsl(var(--input))',
        ring: 'hsl(var(--ring))',
        background: 'hsl(var(--background))',
        foreground: 'hsl(var(--foreground))',
        'subtle-bg': 'hsl(var(--subtle-bg))',
        primary: {
          DEFAULT: 'hsl(var(--primary))',
          foreground: 'hsl(var(--primary-foreground))',
          soft: 'hsl(var(--primary-soft))',
        },
        secondary: {
          DEFAULT: 'hsl(var(--secondary))',
          foreground: 'hsl(var(--secondary-foreground))',
        },
        destructive: {
          DEFAULT: 'hsl(var(--destructive))',
          foreground: 'hsl(var(--destructive-foreground))',
        },
        muted: {
          DEFAULT: 'hsl(var(--muted))',
          foreground: 'hsl(var(--muted-foreground))',
        },
        accent: {
          DEFAULT: 'hsl(var(--accent))',
          foreground: 'hsl(var(--accent-foreground))',
        },
        popover: {
          DEFAULT: 'hsl(var(--popover))',
          foreground: 'hsl(var(--popover-foreground))',
        },
        card: {
          DEFAULT: 'hsl(var(--card))',
          foreground: 'hsl(var(--card-foreground))',
        },
        // Atlas Semantic Status Colors
        success: {
          DEFAULT: 'hsl(142 76% 36%)', /* #16A34A - Green text */
          bg: 'hsl(142 70% 95%)', /* Light green background */
          border: 'hsl(142 70% 85%)', /* Green border */
        },
        warning: {
          DEFAULT: 'hsl(38 92% 50%)', /* #F59E0B - Amber text */
          bg: 'hsl(48 100% 96%)', /* Light amber background */
          border: 'hsl(48 100% 85%)', /* Amber border */
        },
        error: {
          DEFAULT: 'hsl(343 84% 57%)', /* Rose/Red text */
          bg: 'hsl(343 100% 97%)', /* Light red background */
          border: 'hsl(343 100% 90%)', /* Red border */
        },
        info: {
          DEFAULT: 'hsl(217 91% 60%)', /* #3B82F6 - Blue text */
          bg: 'hsl(217 100% 97%)', /* Light blue background */
          border: 'hsl(217 100% 90%)', /* Blue border */
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'BlinkMacSystemFont', '"Segoe UI"', 'sans-serif'],
        mono: ['"JetBrains Mono"', '"SF Mono"', 'ui-monospace', 'Menlo', 'monospace'],
      },
      fontSize: {
        xs: '12px',
        sm: '14px',
        base: '14px', // Base size is 14px per style guide
        lg: '16px',
        xl: '20px',
        '2xl': '24px',
      },
      spacing: {
        // 4px base scale from style guide
        '1': '4px',
        '2': '8px',
        '3': '12px',
        '4': '16px',
        '5': '20px',
        '6': '24px',
        '8': '32px',
        '10': '40px',
      },
      borderRadius: {
        lg: 'var(--radius)', /* 12px for cards */
        md: 'calc(var(--radius) - 2px)', /* 10px */
        sm: '0.5rem', /* 8px for buttons and inputs */
        xl: 'var(--radius)', /* 12px alias */
        full: '9999px', /* Fully rounded */
      },
      boxShadow: {
        // Atlas Depth System Shadows
        card: '0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)', /* Subtle lift for cards */
        hover: '0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)', /* Significant lift on hover */
        modal: '0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1)', /* Floating modals */
        // Keep standard shadcn shadows for compatibility
        sm: '0 1px 2px 0 rgb(0 0 0 / 0.05)',
        DEFAULT: '0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)',
        md: '0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)',
        lg: '0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)',
        xl: '0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1)',
      },
      keyframes: {
        'accordion-down': {
          from: { height: '0' },
          to: { height: 'var(--radix-accordion-content-height)' },
        },
        'accordion-up': {
          from: { height: 'var(--radix-accordion-content-height)' },
          to: { height: '0' },
        },
      },
      animation: {
        'accordion-down': 'accordion-down 0.2s ease-out',
        'accordion-up': 'accordion-up 0.2s ease-out',
      },
    },
  },
  plugins: [require('tailwindcss-animate')],
}

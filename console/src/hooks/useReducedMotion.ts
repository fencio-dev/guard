import { useState, useEffect } from 'react';

const QUERY = '(prefers-reduced-motion: reduce)';

export const useReducedMotion = () => {
  const [matches, setMatches] = useState(window.matchMedia(QUERY).matches);

  useEffect(() => {
    const mediaQuery = window.matchMedia(QUERY);
    const handleChange = () => setMatches(mediaQuery.matches);

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, []);

  return matches;
};

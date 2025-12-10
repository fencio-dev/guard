import React from "react";

export const ThemeProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  // Apply dark mode by default for Fencio v2.0 styling
  React.useEffect(() => {
    document.documentElement.classList.add("dark");
  }, []);

  return <>{children}</>;
};

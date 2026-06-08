export interface ThemeOption {
  id: string;
  name: string;
}

export const THEMES: ThemeOption[] = [
  { id: "dark", name: "Dark" },
  { id: "light", name: "Light" },
  { id: "one-dark", name: "One Dark" },
  { id: "dracula", name: "Dracula" },
  { id: "nord", name: "Nord" },
  { id: "solarized-light", name: "Solarized Light" },
  { id: "rose-pine-dawn", name: "Rosé Pine Dawn" },
];

export const DEFAULT_THEME = "dark";

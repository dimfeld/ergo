import '../src/app.css';
import './preview.css';
export const parameters = {
  actions: { argTypesRegex: '^on[A-Z].*' },
  darkMode: {
    darkClass: 'dark',
    stylePreview: true,
  },
  controls: {
    matchers: {
      color: /(background|color)$/i,
      date: /Date$/,
    },
  },
};

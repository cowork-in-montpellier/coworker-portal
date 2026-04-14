/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {},
  },
  plugins: [require('daisyui')],
  daisyui: {
    // Try swapping to: 'nord', 'corporate', 'dim', 'sunset', 'dracula'
    themes: ['corporate', 'nord', 'dim'],
    defaultTheme: 'nord',
  },
}

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: {
    files: ["./index.html", "./src/**/*.{vue,js,ts,jsx,tsx}"],
  },
  theme: {
    extend: {},
  },
  plugins: [
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    require('daisyui'),
  ],
  daisyui: {
    themes: ["light", "dark"],
  },
}

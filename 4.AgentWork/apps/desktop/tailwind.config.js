/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        brand: {
          400: "var(--brand-primary)",
          500: "var(--brand-primary)",
          600: "var(--brand-hover)",
          700: "var(--brand-hover)",
          800: "var(--brand-light)",
          900: "var(--brand-light)",
          950: "var(--brand-light)",
        },
        accent: {
          400: "var(--brand-primary)",
          500: "var(--brand-primary)",
          600: "var(--brand-hover)",
        },
      },
      backgroundColor: {
        "zinc-950": "var(--tw-bg-zinc-950)",
        "zinc-900": "var(--tw-bg-zinc-900)",
        "zinc-800": "var(--tw-bg-zinc-800)",
        "zinc-700": "var(--tw-bg-zinc-700)",
        "zinc-300": "var(--tw-bg-zinc-300)",
        "zinc-200": "var(--tw-bg-zinc-200)",
        "zinc-100": "var(--tw-bg-zinc-100)",
        "zinc-50":  "var(--tw-bg-zinc-50)",
      },
      borderColor: {
        "zinc-950": "var(--tw-border-zinc-950)",
        "zinc-900": "var(--tw-border-zinc-900)",
        "zinc-800": "var(--tw-border-zinc-800)",
        "zinc-700": "var(--tw-border-zinc-700)",
        "zinc-600": "var(--tw-border-zinc-600)",
        "zinc-500": "var(--tw-border-zinc-500)",
        "zinc-400": "var(--tw-border-zinc-400)",
        "zinc-300": "var(--tw-border-zinc-300)",
        "zinc-200": "var(--tw-border-zinc-200)",
      },
      textColor: {
        "zinc-950": "var(--tw-text-zinc-950)",
        "zinc-900": "var(--tw-text-zinc-900)",
        "zinc-800": "var(--tw-text-zinc-800)",
        "zinc-500": "var(--tw-text-zinc-500)",
        "zinc-400": "var(--tw-text-zinc-400)",
        "zinc-300": "var(--tw-text-zinc-300)",
        "zinc-200": "var(--tw-text-zinc-200)",
        "zinc-100": "var(--tw-text-zinc-100)",
        "zinc-50":  "var(--tw-text-zinc-50)",
      },
    },
  },
  plugins: [],
};

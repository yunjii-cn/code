/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        // 品牌色跟随 CSS 变量（主题切换时自动更新）
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
    },
  },
  plugins: [],
};

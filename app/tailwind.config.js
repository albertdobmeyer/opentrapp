/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        neutral: {
          50: "#f9fafb",
          100: "#f3f4f6",
          200: "#e5e7eb",
          300: "#d1d5db",
          400: "#9ca3af",
          500: "#6b7280",
          600: "#4b5563",
          700: "#374151",
          800: "#1f2937",
          850: "#111827",
          900: "#0b1120",
          950: "#030712",
        },
        // Brand: LobsterTrApp-Green (#009966) — primary identity color
        primary: {
          50: "#e6f5ee",
          200: "#80ccb3",
          400: "#1ab07e",
          500: "#009966",
          600: "#007a52",
          700: "#00583a",
        },
        // Brand: LobsterTrApp-Red (#CC3333) — reserved for the logo shield only.
        // Brand: LobsterTrApp-Blue (#0EA5E9) — secondary accent; pairs with primary in gradients.
        brand: {
          red: "#cc3333",
          "red-dark": "#a82828",
          "red-light": "#d65555",
          blue: "#0ea5e9",
          "blue-dark": "#0284c7",
          "blue-light": "#38bdf8",
        },
        info: {
          400: "#60a5fa",
          500: "#3b82f6",
          600: "#2563eb",
        },
        success: {
          400: "#34d399",
          500: "#10b981",
        },
        warning: {
          400: "#fbbf24",
          500: "#f59e0b",
        },
        danger: {
          400: "#f87171",
          500: "#ef4444",
        },
      },
      fontFamily: {
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          '"SF Pro Text"',
          "Inter",
          "system-ui",
          "sans-serif",
        ],
        display: [
          "-apple-system",
          "BlinkMacSystemFont",
          '"SF Pro Display"',
          "Inter",
          "system-ui",
          "sans-serif",
        ],
        mono: [
          "ui-monospace",
          '"SF Mono"',
          '"Cascadia Code"',
          "Menlo",
          "monospace",
        ],
      },
      spacing: {
        18: "4.5rem",
        22: "5.5rem",
      },
      borderRadius: {
        "2xl": "1.5rem",
      },
      boxShadow: {
        xs: "0 1px 2px 0 rgba(0, 0, 0, 0.2)",
        glow: "0 0 24px rgba(0, 153, 102, 0.15)",
      },
      transitionTimingFunction: {
        "ease-out-soft": "cubic-bezier(0.16, 1, 0.3, 1)",
        "ease-spring": "cubic-bezier(0.34, 1.56, 0.64, 1)",
      },
      animation: {
        "fade-in": "fadeIn 250ms cubic-bezier(0.16, 1, 0.3, 1)",
        "slide-up": "slideUp 250ms cubic-bezier(0.16, 1, 0.3, 1)",
        celebrate: "celebrate 600ms cubic-bezier(0.34, 1.56, 0.64, 1)",
      },
      keyframes: {
        fadeIn: {
          from: { opacity: "0" },
          to: { opacity: "1" },
        },
        slideUp: {
          from: { opacity: "0", transform: "translateY(8px)" },
          to: { opacity: "1", transform: "translateY(0)" },
        },
        celebrate: {
          "0%": { transform: "scale(0.95)", opacity: "0" },
          "50%": { transform: "scale(1.05)", opacity: "1" },
          "100%": { transform: "scale(1)", opacity: "1" },
        },
      },
    },
  },
  plugins: [],
};

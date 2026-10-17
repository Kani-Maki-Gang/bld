/** @type {import('tailwindcss').Config} */
module.exports = {
  content: {
    files: ["*.html", "./src/**/*.rs"],
  },
  theme: {
    extend: {
      colors: {
        canvas: "#0d0e12",
        well: "#0f1116",
        surface: "#14161c",
        hover: "#171a21",
        raised: "#1a1d25",
        chip: "#1c1f28",
        line: {
          faint: "#1a1d24",
          soft: "#1d2129",
          DEFAULT: "#232733",
          strong: "#262a35",
          edge: "#2b3040",
          bright: "#3a4152",
          brighter: "#4a536a",
        },
        ink: {
          DEFAULT: "#e8e9ee",
          soft: "#b9bdc9",
          muted: "#8b90a0",
          faint: "#5d6272",
          ghost: "#3d4250",
        },
        accent: {
          DEFAULT: "#6c5cf5",
          hover: "#7b6cf7",
          link: "#8f83f7",
          ink: "#a89ef9",
          line: "#3d3480",
        },
        ok: "#35c98a",
        bad: {
          DEFAULT: "#f0604f",
          soft: "#f3a399",
          line: "#4a2f30",
          strong: "#7a3a35",
        },
        run: "#4aa8f0",
        warn: {
          DEFAULT: "#f2b544",
          soft: "#c9a055",
          line: "#4a3d22",
        },
      },
      fontFamily: {
        sans: ["'IBM Plex Sans'", "system-ui", "sans-serif"],
        mono: ["'IBM Plex Mono'", "ui-monospace", "monospace"],
      },
      keyframes: {
        "bld-pulse": {
          "0%, 100%": { opacity: "1" },
          "50%": { opacity: ".35" },
        },
        "bld-sweep": {
          "0%": { transform: "translateX(-100%)" },
          "100%": { transform: "translateX(320%)" },
        },
        "bld-caret": {
          "0%, 49%": { opacity: "1" },
          "50%, 100%": { opacity: "0" },
        },
      },
      animation: {
        "bld-pulse": "bld-pulse 1.6s ease-in-out infinite",
        "bld-sweep": "bld-sweep 2.4s linear infinite",
        "bld-caret": "bld-caret 1s step-end infinite",
      },
    },
  },
  plugins: [],
}

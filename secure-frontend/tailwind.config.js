/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./templates/**/*.html",
    "./src/**/*.rs", // In case we have classes in Rust code
  ],
  theme: {
    extend: {
      colors: {
        primary: {
          DEFAULT: '#FF8C42',
          light: '#FF6B35',
        },
        secondary: {
          DEFAULT: '#08415C',
          light: '#0A5A7F',
        },
        success: '#12E193',
        warning: '#FFC857',
        error: '#FF6B6B',
        'text-primary': '#1A202C',
        'text-secondary': '#4A5568',
        'text-tertiary': '#718096',
        'text-muted': '#A0AEC0',
      },
      fontFamily: {
        display: ['"Scope One"', 'serif'],
        sans: ['"Scope One"', 'sans-serif'],
        body: ['"Scope One"', 'serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
      borderRadius: {
        '3xl': '32px',
      },
      backdropBlur: {
        'xl': '20px',
      },
    },
  },
  plugins: [],
}

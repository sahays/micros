# CSS Build Process

This frontend uses **Tailwind CSS CLI** for optimized, production-ready CSS generation.

## Why Tailwind CLI?

- **Tiny bundle size**: Only includes classes actually used in templates (~5-10KB)
- **Automatic purging**: Scans templates and removes unused CSS
- **Built-in minification**: Production-ready output
- **No runtime overhead**: Pre-built CSS, no JavaScript required

## Development Workflow

### First-time setup
```bash
cd secure-frontend
npm install
```

### Build CSS (one-time)
```bash
npm run build:css
```
Generates minified `static/output.css` from `static/input.css`

### Watch mode (during development)
```bash
npm run watch:css
```
Automatically rebuilds CSS when templates or input.css change

## Docker Build

The Dockerfile includes a CSS build stage that:
1. Installs Node.js and Tailwind CLI
2. Scans `templates/**/*.html` for used classes
3. Generates optimized `output.css`
4. Copies only the final CSS to runtime image

No Node.js in the final container - only the optimized CSS file.

## File Structure

```
secure-frontend/
├── static/
│   ├── input.css           # Source CSS with @tailwind directives
│   └── output.css    # Generated output (gitignored if needed)
├── templates/              # Scanned by Tailwind for class usage
├── tailwind.config.js      # Tailwind configuration
├── package.json            # Build scripts
└── Dockerfile              # Multi-stage build with CSS compilation
```

## Custom Styles

All design system components are defined in `static/input.css` using `@layer components`:
- `.card`, `.card-header`, `.card-body`, `.card-footer`
- `.btn-gradient`, `.btn-primary` (alias), `.btn-action`
- `.input-field`, `.input-glass` (alias)
- Status badges, spinners, animations

## Performance

- **Before**: 400KB Tailwind CDN + 600ms runtime parsing
- **After**: ~8KB minified CSS + <10ms load time
- **50-75x smaller bundle** ⚡

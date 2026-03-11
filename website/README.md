# Sqyre website

Static site built with [Hugo](https://gohugo.io/).

## Prerequisites

Install Hugo (extended recommended):

```bash
# via Go
go install -tags extended github.com/gohugoio/hugo/v2@latest

# or download from https://gohugo.io/installation/
```

## Develop locally

From this directory (`website/`):

```bash
hugo server -D
```

Open http://localhost:1313

## Build for production

```bash
hugo --minify
```

Output is in `public/`. Serve that folder with any static host (Netlify, GitHub Pages, etc.).

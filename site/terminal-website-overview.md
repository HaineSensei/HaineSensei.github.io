# Terminal-Style Website Project Overview

## Concept
A minimal, terminal-inspired website with a command-line interface aesthetic. The goal is to create a simple, navigable site that looks like a terminal session but isn't trying to be an actual bash shell.

## Design Philosophy
- **Not a real shell**: We want the terminal aesthetic without setting expectations that bash commands will work
- **Simple navigation**: Limited set of custom commands (help, about, projects, contact, etc.)
- **Minimalist**: Clean, focused interaction - type command, get response
- **No confusion**: Make it clear which commands are available via `help` command

## Current Implementation
A single-file HTML implementation with:
- Seamless input/output styling (input blends with terminal output)
- Basic command handling (help, about, clear)
- Auto-focus on input field
- Auto-scroll to bottom
- Green-on-black terminal color scheme

**File**: `terminal-site.html`

## Technical Stack
- **Pure HTML/CSS/JS**: No frameworks or dependencies needed
- **Client-side only**: All logic runs in browser
- **Static hosting compatible**: Perfect for GitHub Pages

## Key Features Implemented
1. **Seamless Input**: Input field styled to look like part of the output
2. **Command History**: Previous commands and outputs displayed above
3. **Focus Management**: Clicking anywhere keeps input focused
4. **Clear Command**: Ability to clear terminal screen
5. **Command Not Found**: Graceful handling of unknown commands

## Future Enhancements Discussed

### Content Management
- **Separate Content Files**: Store content in separate files rather than hardcoding
- **Markdown Support**: Use `.md` files for content (readable even in raw form)
- **File Structure Example**:
  ```
  /index.html
  /content/
    about.md
    projects.json
    blog-post-1.md
  ```
- **Dynamic Loading**: Use Fetch API to load content files on command

### Command Parsing
- **Argument Parsing**: Support for command flags/arguments (e.g., `command --flag arg`)
- **Subcommands**: Nested command structures
- **Command Registry**: JSON file mapping commands to content files

### Styling Options
- Color scheme customization
- Different terminal themes
- ASCII art/banners
- Custom prompt symbols

## Deployment Plan
- **Platform**: GitHub Pages (static hosting)
- **Repository**: Create `username.github.io` repo or project-specific repo
- **Process**: 
  1. Push HTML file (as `index.html`)
  2. Enable GitHub Pages in repo settings
  3. Live at `username.github.io`
- **Benefits**: Free hosting, free SSL, version controlled, simple deployment

## Content Strategy
- All content is public (no need for server-side logic)
- Markdown files for text content (readable in plain text, parseable for HTML if desired)
- Could display raw markdown (fits terminal aesthetic) or parse to HTML
- Easy to add new commands by adding new content files

## Design Decisions Made
1. **Static over dynamic**: No server needed since all content is public
2. **Client-side parsing**: JavaScript can handle all command logic
3. **Minimal dependencies**: Pure vanilla JS, no build process
4. **Terminal aesthetic without shell complexity**: Look and feel of terminal without trying to emulate bash behavior

## Technical Notes
- JavaScript Fetch API can handle static file loading
- Markdown is readable even without parsing (good for terminal aesthetic)
- Optional: Add lightweight markdown parser from CDN (marked.js, showdown.js) if HTML rendering desired
- localStorage available for client-side state if needed
- Can fetch from external APIs if needed (all still client-side)

## Why This Approach Works
- **Low friction**: Single HTML file to start, easy to extend
- **No maintenance overhead**: No server, database, or backend
- **Fast and reliable**: Static files are fast to serve
- **Version controlled**: Git tracks all changes
- **Flexible**: Can grow from simple to complex while staying static

## Current Status
Basic prototype complete with core functionality working. Ready to extend with:
- Additional commands
- Content file structure
- Markdown content files
- Enhanced command parsing
- Custom styling/themes

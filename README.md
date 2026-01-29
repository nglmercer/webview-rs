# webview-rs

A high-performance N-API binding for creating native webviews and windows in Node.js using [Tao](https://github.com/tauri-apps/tao) and [WRY](https://github.com/tauri-apps/wry).

## Features

- 🪟 **Native Windows** - Create native windows with full control over decorations, transparency, and window state
- 🌐 **WebView Support** - Embed web content using system webviews (WebKit on Linux/macOS, WebView2 on Windows)
- 🎨 **Software Rendering** - Direct pixel buffer rendering without requiring a webview (perfect for games and custom UIs)
- 📐 **Auto-Scaling** - Automatic scaling when window is resized with multiple scale modes
- 🖱️ **Event Handling** - Full mouse, keyboard, and touch event support
- 🎭 **Transparency** - Support for transparent and borderless windows
- 🚀 **High Performance** - Rust-powered with N-API for maximum performance

## Installation

```bash
npm install webview-rs
# or
yarn add webview-rs
# or
bun add webview-rs
```

## Quick Start

### Basic Window

```typescript
import { WindowBuilder, EventLoop, TaoTheme } from 'webview-rs'

const eventLoop = new EventLoop()
const builder = new WindowBuilder()
  .withTitle('My Window')
  .withInnerSize(800, 600)
  .withResizable(true)
  .withTheme(TaoTheme.Dark)

const window = builder.build(eventLoop)

// Run event loop
setInterval(() => {
  eventLoop.runIteration()
}, 16)
```

### WebView

```typescript
import { WebViewBuilder, EventLoop } from 'webview-rs'

const eventLoop = new EventLoop()
const window = new WindowBuilder()
  .withTitle('WebView Example')
  .withInnerSize(1024, 768)
  .build(eventLoop)

const webview = new WebViewBuilder()
  .withUrl('https://example.com')
  .build(window)
```

### Software Rendering with Auto-Scaling

```typescript
import { WindowBuilder, EventLoop, WindowSurface, ScaleMode } from 'webview-rs'

const eventLoop = new EventLoop()
const window = new WindowBuilder()
  .withTitle('Software Render')
  .withInnerSize(800, 600)
  .withResizable(true)  // Enable resizing to see auto-scale in action
  .build(eventLoop)

// Create a surface with fixed logical resolution
// It will auto-scale to the window size
const surface = new WindowSurface(640, 480)
surface.setAutoScale(true)  // Enabled by default
surface.setScaleMode(ScaleMode.Fit)  // Maintain aspect ratio
surface.setBackgroundColor(20, 20, 30, 255)  // Dark background

// In your render loop:
const pixelBuffer = new Uint8Array(640 * 480 * 4)  // RGBA
// ... fill pixelBuffer with your rendering ...

// Render with automatic scaling
surface.renderToWindow(window, Buffer.from(pixelBuffer))
```

## Auto-Scaling Feature

The `WindowSurface` class provides automatic scaling when the window is resized. This is perfect for games and applications that render to a fixed-resolution buffer.

### Scale Modes

| Mode | Description |
|------|-------------|
| `ScaleMode.Fit` | Maintain aspect ratio with letterboxing/pillarboxing (black bars) |
| `ScaleMode.Fill` | Maintain aspect ratio and crop to fill the entire window |
| `ScaleMode.Stretch` | Stretch the buffer to fit the window (may distort aspect ratio) |
| `ScaleMode.Integer` | Integer scaling for pixel-perfect rendering (2x, 3x, etc.) |
| `ScaleMode.None` | No scaling, keep original size centered |

### Example: Cycling Scale Modes

```typescript
const scaleModes = [
  ScaleMode.Fit,
  ScaleMode.Fill,
  ScaleMode.Stretch,
  ScaleMode.Integer,
  ScaleMode.None
]

let currentMode = 0

// Switch scale mode every 5 seconds
setInterval(() => {
  currentMode = (currentMode + 1) % scaleModes.length
  surface.setScaleMode(scaleModes[currentMode])
  console.log(`Scale mode: ${ScaleMode[scaleModes[currentMode]]}`)
}, 5000)
```

## Examples

See the [`examples/`](examples/) directory for more examples:

- [`basic-window-example.ts`](examples/basic-window-example.ts) - Basic window creation
- [`tao-rainbow-render-example.ts`](examples/tao-rainbow-render-example.ts) - Software rendering with auto-scaling demo
- [`basic-webview-example.ts`](examples/basic-webview-example.ts) - Basic webview usage
- [`ipc-example.ts`](examples/ipc-example.ts) - Inter-process communication
- [`transparency.ts`](examples/transparency.ts) - Transparent window example

Run an example:

```bash
bun run examples/tao-rainbow-render-example.ts
```

## API Reference

### WindowSurface

A software rendering surface with auto-scaling support.

#### Constructor

```typescript
new WindowSurface(width: number, height: number): WindowSurface
```

#### Methods

| Method | Description |
|--------|-------------|
| `resize(width, height)` | Update the window dimensions for scaling calculations |
| `resizeBuffer(width, height)` | Change the logical buffer dimensions |
| `setAutoScale(enabled)` | Enable/disable auto-scaling (default: true) |
| `setScaleMode(mode)` | Set the scale mode (Fit, Fill, Stretch, Integer, None) |
| `setBackgroundColor(r, g, b, a)` | Set the background color for letterboxing |
| `renderToWindow(window, buffer)` | Render the pixel buffer with auto-scaling |

#### Properties

| Property | Type | Description |
|----------|------|-------------|
| `width` | `number` | Logical buffer width |
| `height` | `number` | Logical buffer height |
| `windowWidth` | `number` | Current window width |
| `windowHeight` | `number` | Current window height |
| `isAutoScaleEnabled` | `boolean` | Whether auto-scaling is enabled |
| `scaleMode` | `ScaleMode` | Current scale mode |
| `scaleFactor` | `number` | Current scale factor |

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/)
- [Node.js](https://nodejs.org/) or [Bun](https://bun.sh/)

### Build

```bash
# Install dependencies
bun install

# Build the native module
cargo build --release

# Or use the CLI
bun run cli/build.mjs
```

## Platform Support

| Platform | Status |
|----------|--------|
| Linux (X11/Wayland) | ✅ Supported |
| macOS | ✅ Supported |
| Windows | ✅ Supported |

## License

MIT License - See [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- [Tao](https://github.com/tauri-apps/tao) - Cross-platform windowing library
- [WRY](https://github.com/tauri-apps/wry) - Webview library
- [NAPI-RS](https://napi.rs/) - Rust bindings for Node.js

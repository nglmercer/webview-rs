/**
 * Tao Rainbow Buffer Render Example
 *
 * Demonstrates creating a native window and rendering an animated rainbow
 * directly to a pixel buffer. This shows how to implement custom rendering
 * without using a webview - perfect for games, animations, or custom UIs.
 *
 * This example uses a software renderer that generates RGBA pixel data and
 * displays it using the window's render() method.
 *
 * NEW: Auto-scale feature demonstration - the rendering automatically adapts
 * to window size changes with multiple scale modes (Fit, Fill, Stretch, Integer, None).
 */

import { WindowBuilder, EventLoop, TaoTheme, WindowSurface, ScaleMode } from '../index.js'
import { createLogger } from './logger.js'

const logger = createLogger('TaoRainbowRender')

/**
 * Software renderer that generates animated rainbow patterns
 */
class RainbowRenderer {
  private width: number = 800
  private height: number = 600
  private pixelBuffer: Uint8ClampedArray = new Uint8ClampedArray()
  private animationTime: number = 0
  private hueOffset: number = 0

  constructor(width: number, height: number) {
    this.width = width
    this.height = height
    this.pixelBuffer = new Uint8ClampedArray(width * height * 4) // RGBA
    logger.info(`Renderer initialized: ${width}x${height}`)
  }

  /**
   * Resize the render buffer
   */
  resize(width: number, height: number): void {
    this.width = width
    this.height = height
    this.pixelBuffer = new Uint8ClampedArray(width * height * 4)
    logger.info(`Renderer resized: ${width}x${height}`)
  }

  /**
   * Get current buffer dimensions
   */
  getDimensions(): { width: number; height: number } {
    return { width: this.width, height: this.height }
  }

  /**
   * Convert HSL to RGB
   */
  private hslToRgb(h: number, s: number, l: number): [number, number, number] {
    const c = (1 - Math.abs(2 * l - 1)) * s
    const x = c * (1 - Math.abs(((h / 60) % 2) - 1))
    const m = l - c / 2

    let r = 0, g = 0, b = 0

    if (h >= 0 && h < 60) {
      r = c; g = x; b = 0
    } else if (h >= 60 && h < 120) {
      r = x; g = c; b = 0
    } else if (h >= 120 && h < 180) {
      r = 0; g = c; b = x
    } else if (h >= 180 && h < 240) {
      r = 0; g = x; b = c
    } else if (h >= 240 && h < 300) {
      r = x; g = 0; b = c
    } else {
      r = c; g = 0; b = x
    }

    return [
      Math.round((r + m) * 255),
      Math.round((g + m) * 255),
      Math.round((b + m) * 255)
    ]
  }

  /**
   * Render a horizontal rainbow gradient
   */
  renderHorizontalRainbow(): Uint8ClampedArray {
    const { width, height } = this
    const buffer = this.pixelBuffer

    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        // Calculate hue based on x position and animation offset
        const hue = ((x / width) * 360 + this.hueOffset) % 360
        const [r, g, b] = this.hslToRgb(hue, 1.0, 0.5)

        const index = (y * width + x) * 4
        buffer[index] = r     // R
        buffer[index + 1] = g // G
        buffer[index + 2] = b // B
        buffer[index + 3] = 255 // A (fully opaque)
      }
    }

    return buffer
  }

  /**
   * Render a vertical rainbow gradient
   */
  renderVerticalRainbow(): Uint8ClampedArray {
    const { width, height } = this
    const buffer = this.pixelBuffer

    for (let y = 0; y < height; y++) {
      const hue = ((y / height) * 360 + this.hueOffset) % 360
      const [r, g, b] = this.hslToRgb(hue, 1.0, 0.5)

      for (let x = 0; x < width; x++) {
        const index = (y * width + x) * 4
        buffer[index] = r
        buffer[index + 1] = g
        buffer[index + 2] = b
        buffer[index + 3] = 255
      }
    }

    return buffer
  }

  /**
   * Render a circular/radial rainbow pattern
   */
  renderRadialRainbow(): Uint8ClampedArray {
    const { width, height } = this
    const buffer = this.pixelBuffer
    const centerX = width / 2
    const centerY = height / 2
    const maxDist = Math.sqrt(centerX * centerX + centerY * centerY)

    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const dx = x - centerX
        const dy = y - centerY
        const dist = Math.sqrt(dx * dx + dy * dy)
        
        const hue = ((dist / maxDist) * 360 + this.hueOffset) % 360
        const [r, g, b] = this.hslToRgb(hue, 1.0, 0.5)

        const index = (y * width + x) * 4
        buffer[index] = r
        buffer[index + 1] = g
        buffer[index + 2] = b
        buffer[index + 3] = 255
      }
    }

    return buffer
  }

  /**
   * Render an animated wave pattern with rainbow colors
   */
  renderWavePattern(): Uint8ClampedArray {
    const { width, height } = this
    const buffer = this.pixelBuffer
    const time = this.animationTime * 0.002

    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        // Create wave pattern
        const wave1 = Math.sin(x * 0.02 + time * 2) * 50
        const wave2 = Math.cos(y * 0.02 + time * 1.5) * 50
        const wave3 = Math.sin((x + y) * 0.01 + time) * 30
        
        const combined = wave1 + wave2 + wave3
        const hue = ((combined + 150) / 300 * 360 + this.hueOffset) % 360
        const [r, g, b] = this.hslToRgb(hue, 0.8, 0.5)

        const index = (y * width + x) * 4
        buffer[index] = r
        buffer[index + 1] = g
        buffer[index + 2] = b
        buffer[index + 3] = 255
      }
    }

    return buffer
  }

  /**
   * Render a spiral rainbow pattern
   */
  renderSpiralPattern(): Uint8ClampedArray {
    const { width, height } = this
    const buffer = this.pixelBuffer
    const centerX = width / 2
    const centerY = height / 2
    const time = this.animationTime * 0.001

    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const dx = x - centerX
        const dy = y - centerY
        const angle = Math.atan2(dy, dx)
        const dist = Math.sqrt(dx * dx + dy * dy)
        
        const spiral = angle * (180 / Math.PI) + dist * 0.5 - time * 100
        const hue = (spiral + this.hueOffset) % 360
        const [r, g, b] = this.hslToRgb(Math.abs(hue), 1.0, 0.5)

        const index = (y * width + x) * 4
        buffer[index] = r
        buffer[index + 1] = g
        buffer[index + 2] = b
        buffer[index + 3] = 255
      }
    }

    return buffer
  }

  /**
   * Update animation state
   */
  update(deltaTime: number): void {
    this.animationTime += deltaTime
    this.hueOffset = (this.hueOffset + deltaTime * 0.05) % 360
  }

  /**
   * Get current pixel buffer (for display)
   */
  getBuffer(): Uint8ClampedArray {
    return this.pixelBuffer
  }
}

/**
 * Window manager with integrated rainbow renderer and auto-scaling support
 */
class RainbowWindowManager {
  private window: any = null
  private renderer: RainbowRenderer | null = null
  private surface: WindowSurface | null = null
  private frameCount: number = 0
  private lastFrameTime: number = Date.now()
  private fps: number = 0
  private currentPattern: number = 0
  private currentScaleMode: number = 0
  private readonly patterns = [
    'horizontal',
    'vertical',
    'radial',
    'wave',
    'spiral'
  ] as const

  private readonly scaleModes = [
    { name: 'Fit', mode: ScaleMode.Fit, desc: 'Maintain aspect ratio with black bars' },
    { name: 'Fill', mode: ScaleMode.Fill, desc: 'Maintain aspect ratio, crop to fill' },
    { name: 'Stretch', mode: ScaleMode.Stretch, desc: 'Stretch to fill window' },
    { name: 'Integer', mode: ScaleMode.Integer, desc: 'Pixel-perfect integer scaling' },
    { name: 'None', mode: ScaleMode.None, desc: 'No scaling, centered' }
  ] as const

  /**
   * Create window and renderer
   */
  async createWindow(eventLoop: EventLoop): Promise<any> {
    logger.section('Creating Rainbow Render Window')
    logger.info('This window renders animated rainbow patterns!')
    logger.info('No webview - pure software rendering to pixel buffer')
    logger.info('')
    logger.info('🎯 NEW: Auto-scale feature enabled by default!')
    logger.info('   The rendering automatically adapts to window size changes.')

    const builder = new WindowBuilder()
      .withTitle('🌈 Rainbow Render - Tao Only (Auto-Scale Demo)')
      .withInnerSize(800, 600)
      .withPosition(100, 100)
      .withResizable(true)
      .withDecorated(true)
      .withVisible(true)
      .withFocused(true)
      .withTheme(TaoTheme.Dark)

    this.window = builder.build(eventLoop)

    // Initialize renderer with fixed buffer size (logical resolution)
    // The surface will handle scaling to the actual window size
    const logicalWidth = 640
    const logicalHeight = 480
    this.renderer = new RainbowRenderer(logicalWidth, logicalHeight)

    // Create WindowSurface with auto-scaling enabled (default)
    this.surface = new WindowSurface(logicalWidth, logicalHeight)
    this.surface.setAutoScale(true)
    this.surface.setScaleMode(ScaleMode.Fit)
    this.surface.setBackgroundColor(20, 20, 30, 255) // Dark blue-gray background

    // Get actual window size
    const size = this.window.innerSize()
    this.surface.resize(Math.floor(size.width), Math.floor(size.height))

    logger.success('Window and renderer created', {
      windowId: this.window.id,
      title: this.window.title(),
      logicalSize: `${logicalWidth}x${logicalHeight}`,
      windowSize: `${size.width}x${size.height}`,
      autoScale: this.surface.isAutoScaleEnabled,
      scaleMode: this.scaleModes[this.currentScaleMode].name,
      patterns: this.patterns
    })

    return this.window
  }

  /**
   * Render current frame to the window using WindowSurface with auto-scaling
   */
  render(): void {
    if (!this.renderer || !this.window || !this.surface) return

    const pattern = this.patterns[this.currentPattern]

    switch (pattern) {
      case 'horizontal':
        this.renderer.renderHorizontalRainbow()
        break
      case 'vertical':
        this.renderer.renderVerticalRainbow()
        break
      case 'radial':
        this.renderer.renderRadialRainbow()
        break
      case 'wave':
        this.renderer.renderWavePattern()
        break
      case 'spiral':
        this.renderer.renderSpiralPattern()
        break
    }

    // Render the pixel buffer to the window using WindowSurface
    // The surface automatically handles scaling based on current window size
    const buffer = this.renderer.getBuffer()

    // Convert Uint8ClampedArray to Buffer for N-API
    const nodeBuffer = Buffer.alloc(buffer.length)
    for (let i = 0; i < buffer.length; i++) {
      nodeBuffer[i] = buffer[i]
    }

    // Use WindowSurface for rendering with auto-scaling
    this.surface.renderToWindow(this.window, nodeBuffer)
  }

  /**
   * Update animation
   */
  update(): void {
    if (!this.renderer) return

    const now = Date.now()
    const delta = now - this.lastFrameTime
    this.renderer.update(delta)

    // Calculate FPS
    this.frameCount++
    if (delta >= 1000) {
      this.fps = Math.round((this.frameCount * 1000) / delta)
      const scaleMode = this.scaleModes[this.currentScaleMode]
      logger.debug(`Pattern: ${this.patterns[this.currentPattern]} | Scale: ${scaleMode.name} | FPS: ${this.fps}`)
      this.frameCount = 0
      this.lastFrameTime = now
    }
  }

  /**
   * Switch to next pattern
   */
  nextPattern(): void {
    this.currentPattern = (this.currentPattern + 1) % this.patterns.length
    logger.info(`Switched to pattern: ${this.patterns[this.currentPattern]}`)
  }

  /**
   * Switch to next scale mode
   */
  nextScaleMode(): void {
    if (!this.surface) return
    this.currentScaleMode = (this.currentScaleMode + 1) % this.scaleModes.length
    const mode = this.scaleModes[this.currentScaleMode]
    this.surface.setScaleMode(mode.mode)
    logger.info(`Switched to scale mode: ${mode.name} - ${mode.desc}`)
  }

  /**
   * Toggle auto-scale on/off
   */
  toggleAutoScale(): void {
    if (!this.surface) return
    const newValue = !this.surface.isAutoScaleEnabled
    this.surface.setAutoScale(newValue)
    logger.info(`Auto-scale ${newValue ? 'enabled' : 'disabled'}`)
  }

  /**
   * Get current pattern name
   */
  getCurrentPattern(): string {
    return this.patterns[this.currentPattern]
  }

  /**
   * Get current scale mode name
   */
  getCurrentScaleMode(): string {
    return this.scaleModes[this.currentScaleMode].name
  }

  /**
   * Request window redraw
   */
  requestRedraw(): void {
    if (this.window) {
      this.window.requestRedraw()
    }
  }

  /**
   * Handle window resize - WindowSurface automatically handles scaling
   * We just need to update the surface with new window dimensions
   */
  handleResize(): void {
    if (!this.window || !this.surface) return

    const size = this.window.innerSize()
    this.surface.resize(Math.floor(size.width), Math.floor(size.height))

    const dims = this.renderer?.getDimensions()
    logger.info(`Window resized: ${size.width}x${size.height} (buffer: ${dims?.width}x${dims?.height})`)
  }

  /**
   * Get renderer stats
   */
  getStats(): any {
    if (!this.renderer || !this.surface) return null

    return {
      pattern: this.getCurrentPattern(),
      scaleMode: this.getCurrentScaleMode(),
      autoScale: this.surface.isAutoScaleEnabled,
      fps: this.fps,
      logicalDimensions: this.renderer.getDimensions(),
      windowDimensions: {
        width: this.surface.windowWidth,
        height: this.surface.windowHeight
      },
      scaleFactor: this.surface.scaleFactor,
      bufferSize: this.renderer.getBuffer().length
    }
  }

  /**
   * Log current stats
   */
  logStats(): void {
    const stats = this.getStats()
    if (stats) {
      logger.object('Render Stats', stats)
    }
  }
}

/**
 * Main function
 */
async function main() {
  logger.banner(
    'Animated pixel buffer rendering with tao-only window'
  )

  try {
    // Create event loop
    logger.info('Creating event loop...')
    const eventLoop = new EventLoop()
    logger.success('Event loop created')

    // Create window manager with rainbow renderer
    const manager = new RainbowWindowManager()
    const window = await manager.createWindow(eventLoop)

    // Log initial stats
    manager.logStats()

    // Cycle patterns every 3 seconds
    setInterval(() => {
      manager.nextPattern()
    }, 3000)

    // Cycle scale modes every 5 seconds
    setInterval(() => {
      manager.nextScaleMode()
    }, 5000)

    // Log stats every 5 seconds
    setInterval(() => {
      manager.logStats()
    }, 5000)

    // Start render loop
    logger.section('Starting Render Loop')
    logger.info('Press Ctrl+C to exit')

    // Render loop
    // NOTE: Running at 60 FPS (16ms) with softbuffer can cause "Maximum number of clients reached"
    // errors on Wayland/X11 systems because softbuffer creates a new display connection on each render.
    // Using 30 FPS (33ms) or lower reduces the connection churn and prevents crashes.
    const renderInterval = setInterval(() => {
      manager.update()
      manager.render()
      manager.requestRedraw()
    }, 33) // ~30 FPS - reduces display server connection churn

    // Run event loop
    const interval = setInterval(() => {
      const running = eventLoop.runIteration()

      if (!running) {
        clearInterval(interval)
        clearInterval(renderInterval)
        logger.info('Event loop ended')
        process.exit(0)
      }

      // Keep window reference alive
      void window.id
    }, 10)

  } catch (error) {
    logger.error('Error in rainbow render example', {
      error: error instanceof Error ? error.message : String(error),
      stack: error instanceof Error ? error.stack : undefined
    })
    process.exit(1)
  }
}

main()

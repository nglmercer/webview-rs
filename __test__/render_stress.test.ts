import { describe, test, expect, beforeAll, afterAll } from 'bun:test'
import { WindowBuilder, EventLoop, PixelRenderer, RenderOptions, ScaleMode } from '../index.js'

// Ensure tests run sequentially to avoid GTK D-Bus conflicts
// GTK applications can only own the org.gtk.Application interface once at a time
const GTK_INIT_TIMEOUT = 5000
let gtkAvailable = false // Default to false to avoid crashes

// Shared event loop for all tests on Linux (GTK only allows one per process)
// This must be at module level to persist across tests
let sharedEventLoop: EventLoop | null = null

// Track if we've already tried to create an event loop (even if it failed)
let eventLoopCreationAttempted = false

/**
 * Get or create the shared EventLoop for Linux/GTK.
 * GTK only allows one EventLoop per process, so all tests must share it.
 */
function getOrCreateEventLoop(): EventLoop {
  if (!sharedEventLoop && !eventLoopCreationAttempted) {
    eventLoopCreationAttempted = true
    try {
      sharedEventLoop = new EventLoop()
    } catch (e) {
      // If we get the "Only one EventLoop" error, it means one was already
      // created in beforeAll during the GTK availability check
      if (e instanceof Error && e.message.includes('Only one EventLoop')) {
        // The shared event loop should have been set by checkGtkAvailability
        // If not, we have a problem
        if (!sharedEventLoop) {
          throw new Error('EventLoop was created in checkGtkAvailability but not stored. Tests must run sequentially.')
        }
      } else {
        throw e
      }
    }
  }
  if (!sharedEventLoop) {
    throw new Error('EventLoop not available - GTK only allows one per process')
  }
  return sharedEventLoop
}

/**
 * Check if GTK is available on this system
 * Uses a unique app ID to avoid D-Bus conflicts with existing GTK applications
 */
async function checkGtkAvailability(): Promise<boolean> {
  try {
    // Set a unique GTK application ID to avoid D-Bus conflicts
    // This prevents "Ya existe un objeto exportado" errors when multiple tests run
    const uniqueId = `webview_rs_test_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`
    process.env.G_APPLICATION_ID = uniqueId
    process.env.GDK_BACKEND = process.env.GDK_BACKEND || 'wayland,x11'
    
    // Quick GTK check - just try creating an event loop
    // On Linux, this will also initialize our global EventLoop tracking
    // Store the event loop in the shared variable so tests can use it
    eventLoopCreationAttempted = true
    sharedEventLoop = new EventLoop()
    const testWindow = new WindowBuilder()
      .withTitle('GTK Check')
      .withInnerSize(100, 100)
      .build(sharedEventLoop)
    testWindow.close()
    return true
  } catch (e) {
    // If the error is about only one EventLoop per process, that's actually
    // a success case - it means GTK is available but we already created one
    if (e instanceof Error && e.message.includes('Only one EventLoop')) {
      return true
    }
    return false
  }
}

beforeAll(async () => {
  // Check if running in CI - skip GTK tests in CI environments
  const isCI = process.env.CI === 'true' || process.env.GITHUB_ACTIONS === 'true'
  if (isCI) {
    console.warn('Running in CI environment, skipping GTK tests')
    gtkAvailable = false
    return
  }
  
  // Check if DISPLAY or WAYLAND_DISPLAY is available
  const hasDisplay = process.env.DISPLAY || process.env.WAYLAND_DISPLAY
  if (!hasDisplay) {
    console.warn('No display available (DISPLAY or WAYLAND_DISPLAY not set), skipping GTK tests')
    gtkAvailable = false
    return
  }
  
  // Check for D-Bus session - required for GTK on Wayland
  const hasDBus = process.env.DBUS_SESSION_BUS_ADDRESS
  if (!hasDBus) {
    console.warn('No D-Bus session available, skipping GTK tests')
    gtkAvailable = false
    return
  }
  
  // Check if there's already a conflicting GTK application registered on D-Bus
  // This is the main cause of "An object is already exported for the interface org.gtk.Application" errors
  // Note: busctl may fail in some environments (e.g., containers), so we handle that gracefully
  try {
    const proc = Bun.spawn(['busctl', '--user', 'status', 'org.gtk.Application'], {
      stdout: 'ignore',
      stderr: 'ignore'
    })
    const exitCode = await proc.exited
    if (exitCode === 0) {
      console.warn('Another GTK application is already registered on D-Bus, skipping GTK tests to avoid conflicts')
      gtkAvailable = false
      return
    }
  } catch (e) {
    // busctl not available or command failed - this is common in containerized environments
    // We'll still try to run the GTK availability check
    console.log('Note: Could not check D-Bus status, will attempt GTK check anyway')
  }
  
  // Try to detect if there's already a GTK application running that might conflict
  // This is a heuristic - if the test fails with D-Bus errors, we'll skip
  // NOTE: The checkGtkAvailability() function creates an EventLoop which can crash the process
  // if GTK is already initialized by another application. We skip this check in the beforeAll
  // and instead rely on the skipIf(!gtkAvailable) in the tests.
  // To enable these tests, set WEBVIEW_RS_ENABLE_GTK_TESTS=1 environment variable
  if (process.env.WEBVIEW_RS_ENABLE_GTK_TESTS === '1') {
    try {
      gtkAvailable = await checkGtkAvailability()
      if (!gtkAvailable) {
        console.warn('GTK not available, skipping render stress tests')
      }
    } catch (e) {
      console.warn('GTK initialization failed, skipping render stress tests:', e)
      gtkAvailable = false
    }
  } else {
    console.log('GTK tests disabled by default. Set WEBVIEW_RS_ENABLE_GTK_TESTS=1 to enable.')
    gtkAvailable = false
  }
}, GTK_INIT_TIMEOUT)

/**
 * Stress test for render functionality
 * Tests high frame counts with large buffer sizes
 * Reproduces issue: "Maximum number of clients reached" after ~231 frames
 * 
 * NOTE: On Linux/GTK, only ONE EventLoop can be created per process.
 * This is a fundamental GTK limitation. Tests must share a single EventLoop.
 */
describe('Render Stress Tests', () => {
  test('should handle 250+ frames without resource exhaustion', async () => {
    const width = 1920  // High resolution width
    const height = 1080 // High resolution height
    const frameCount = 10000 // More than 231 to trigger the issue
    
    // Create buffers (RGBA format) - ~8MB per buffer
    const buffer = Buffer.alloc(width * height * 4)
    
    // Fill with gradient pattern
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const idx = (y * width + x) * 4
        buffer[idx] = (x / width) * 255     // R
        buffer[idx + 1] = (y / height) * 255 // G
        buffer[idx + 2] = 128                // B
        buffer[idx + 3] = 255                // A
      }
    }
    
    // Use shared event loop on Linux, create new one on other platforms
    const eventLoop = process.platform === 'linux' ? getOrCreateEventLoop() : new EventLoop()
    const window = new WindowBuilder()
      .withTitle('Render Stress Test - 250+ frames')
      .withInnerSize(width / 2, height / 2) // Display at half resolution
      .build(eventLoop)
    
    // Create pixel renderer
    const options: RenderOptions = {
      bufferWidth: width,
      bufferHeight: height,
      scaleMode: ScaleMode.Fit,
      backgroundColor: [0, 0, 0, 255]
    }
    const renderer = PixelRenderer.withOptions(options)
    
    let renderedFrames = 0
    let errors: Error[] = []
    
    // Run event loop iteration to initialize window
    eventLoop.runIteration()
    
    // Render frames
    for (let i = 0; i < frameCount; i++) {
      try {
        // Update buffer with frame number for visual feedback
        const frameNumber = i + 1
        for (let j = 0; j < 100; j++) {
          const idx = j * 4
          buffer[idx] = (frameNumber * 10) % 255
        }
        
        renderer.render(window, buffer)
        renderedFrames++
        
        // Log every 50 frames
        if (frameNumber % 50 === 0) {
          console.log(`Frame ${frameNumber} rendered successfully, data len: ${buffer.length}`)
        }
        
        // Run event loop iteration
        eventLoop.runIteration()
      } catch (error) {
        errors.push(error as Error)
        console.error(`Error at frame ${i + 1}:`, error)
        break
      }
    }
    
    // Cleanup
    try {
      window.close()
    } catch (e) {
      // Ignore cleanup errors
    }
    
    // Assertions
    expect(errors).toHaveLength(0)
    expect(renderedFrames).toBe(frameCount)
  }, 60000) // 60 second timeout
  
  test('should handle multiple renderers without client limit error', async () => {
    const width = 800
    const height = 600
    const rendererCount = 10
    
    // Use shared event loop on Linux, create new one on other platforms
    const eventLoop = process.platform === 'linux' ? getOrCreateEventLoop() : new EventLoop()
    const window = new WindowBuilder()
      .withTitle('Multi-renderer Test')
      .withInnerSize(width, height)
      .build(eventLoop)
    
    const buffer = Buffer.alloc(width * height * 4)
    buffer.fill(255) // White
    
    eventLoop.runIteration()
    
    const renderers: PixelRenderer[] = []
    
    for (let i = 0; i < rendererCount; i++) {
      const options: RenderOptions = {
        bufferWidth: width,
        bufferHeight: height,
        scaleMode: ScaleMode.Fit,
        backgroundColor: [i * 20, i * 20, i * 20, 255]
      }
      renderers.push(PixelRenderer.withOptions(options))
    }
    
    let errors: Error[] = []
    
    // Render with each renderer 30 times
    for (let r = 0; r < renderers.length; r++) {
      for (let f = 0; f < 30; f++) {
        try {
          renderers[r].render(window, buffer)
          eventLoop.runIteration()
        } catch (error) {
          errors.push(error as Error)
          console.error(`Error with renderer ${r} at frame ${f}:`, error)
          break
        }
      }
    }
    
    try {
      window.close()
    } catch (e) {
      // Ignore cleanup errors
    }
    
    expect(errors).toHaveLength(0)
  }, 30000)
})

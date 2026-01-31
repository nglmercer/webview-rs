/**
 * Simple Render Example - Black/White Frames
 *
 * A minimal example demonstrating pixel buffer rendering
 * with alternating black and white frames
 */

import { WindowBuilder, EventLoop, PixelRenderer, RenderOptions, ScaleMode } from '../index.js'

// Create black and white pixel buffers (RGBA format)
const width = 400
const height = 300

const blackBuffer = Buffer.alloc(width * height * 4)
const whiteBuffer = Buffer.alloc(width * height * 4)

// Fill black buffer (0, 0, 0, 255)
for (let i = 0; i < blackBuffer.length; i += 4) {
  blackBuffer[i] = 0
  blackBuffer[i + 1] = 0
  blackBuffer[i + 2] = 0
  blackBuffer[i + 3] = 255
}

// Fill white buffer (255, 255, 255, 255)
for (let i = 0; i < whiteBuffer.length; i += 4) {
  whiteBuffer[i] = 255
  whiteBuffer[i + 1] = 255
  whiteBuffer[i + 2] = 255
  whiteBuffer[i + 3] = 255
}

// Create event loop and window
const eventLoop = new EventLoop()
const window = new WindowBuilder()
  .withTitle('Simple Render - Black/White')
  .withInnerSize(width, height)
  .withForceWayland(true)
  .build(eventLoop)

// Create pixel renderer with options
const options: RenderOptions = {
  bufferWidth: width,
  bufferHeight: height,
  scaleMode: ScaleMode.Fit,
  backgroundColor: [0, 0, 0, 255]
}
const renderer = PixelRenderer.withOptions(options)

// Render alternating frames (2 frames total)
let frame = 0
const maxFrames = 2

const renderFrame = () => {
  if (frame >= maxFrames) {
    console.log('✓ Rendering complete!')
    return
  }

  const buffer = frame % 2 === 0 ? blackBuffer : whiteBuffer
  const color = frame % 2 === 0 ? 'BLACK' : 'WHITE'

  try {
    renderer.render(window, buffer)
    console.log(`Frame ${frame + 1}/${maxFrames}: ${color}`)
    frame++
    // Schedule next frame after a delay
    setTimeout(renderFrame, 500)
  } catch (error) {
    console.error('Render error:', error)
  }
}

console.log('Starting render example...')
console.log('Press Ctrl+C to exit')

// Keep the event loop running - MUST start polling BEFORE rendering
const poll = () => {
    if (eventLoop.runIteration()) {
        void window.id;
        setTimeout(poll, 16); // ~60fps
    } else {
        process.exit(0);
    }
};

// Start the event loop polling first
poll()

// Start rendering after a short delay to let the window initialize
setTimeout(renderFrame, 100)

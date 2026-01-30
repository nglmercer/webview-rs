/**
 * Window Event Listener Example
 *
 * Demonstrates using the on_event callback to receive window events
 * from the EventLoop using @webviewjs/webview
 */

import { WindowBuilder, EventLoop, WindowEvent } from '../index.js'
import { createLogger } from './logger.js'

const logger = createLogger('EventListener')

/**
 * Main function to run the event listener example
 */
async function main() {
  logger.banner('Window Event Listener Example', 'Demonstrating window event callbacks with on_event')

  try {
    logger.info('Creating event loop...')
    const eventLoop = new EventLoop()
    logger.success('Event loop created')

    // Register the event handler callback
    logger.info('Registering event handler...')
    eventLoop.onEvent((_e,eventData) => {

      const { event, windowId } = eventData
      logger.info(`Received window event: ${event}`, { windowId, event })

      // Handle specific events
      switch (event) {
        case WindowEvent.CloseRequested:
          logger.info('Window close requested - application will exit')
          break
        case WindowEvent.Resized:
          logger.debug('Window was resized')
          break
        case WindowEvent.Moved:
          logger.debug('Window was moved')
          break
        case WindowEvent.Focused:
          logger.info('Window gained focus')
          break
        case WindowEvent.Unfocused:
          logger.info('Window lost focus')
          break
        case WindowEvent.Destroyed:
          logger.info('Window was destroyed')
          break
        case WindowEvent.ScaleFactorChanged:
          logger.debug('Window scale factor changed')
          break
        case WindowEvent.ThemeChanged:
          logger.info('Window theme changed')
          break
        default:
          logger.debug(`Unhandled event: ${event}`)
      }
    })
    logger.success('Event handler registered')

    logger.section('Creating Window')
    const builder = new WindowBuilder()
      .withTitle('Event Listener Demo')
      .withInnerSize(800, 600)
      .withPosition(100, 100)
      .withResizable(true)
      .withDecorated(true)
      .withVisible(true)
      .withFocused(true)

    const window = builder.build(eventLoop)
    logger.success('Window created', { windowId: window.id })

    logger.section('Starting Event Loop (run_iteration mode)')
    logger.info('Events will be logged as they occur')
    logger.info('Try resizing, moving, or closing the window')
    logger.info('Press Ctrl+C to exit')

    // Run using run_iteration to demonstrate event handling
    let running = true
    while (running) {
      running = eventLoop.runIteration()
      // Small delay to prevent CPU spinning
      await new Promise(resolve => setTimeout(resolve, 16))
    }

    logger.info('Event loop ended')

  } catch (error) {
    logger.error('Error executing event listener example', {
      error: error instanceof Error ? error.message : String(error),
      stack: error instanceof Error ? error.stack : undefined
    })
    process.exit(1)
  }
}

main()

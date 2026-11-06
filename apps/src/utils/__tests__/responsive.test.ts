import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import {
  getCurrentFontSize,
  getCurrentScale,
  initResponsive,
  pxToRem,
  pxToResponsive,
  setRem,
} from '../responsive'

describe('responsive.ts', () => {
  const originalInnerWidth = window.innerWidth
  const originalGetComputedStyle = window.getComputedStyle
  const originalAddEventListener = window.addEventListener
  const listeners = new Map<string, () => void>()

  beforeEach(() => {
    vi.useFakeTimers()
    listeners.clear()

    Object.defineProperty(window, 'innerWidth', {
      value: 1920,
      writable: true,
      configurable: true,
    })

    Object.defineProperty(document, 'documentElement', {
      value: { style: { fontSize: '100px' } },
      configurable: true,
    })

    window.getComputedStyle = vi.fn(() => ({
      fontSize: document.documentElement.style.fontSize,
    })) as never
    window.addEventListener = vi.fn((event, handler) => {
      listeners.set(event as string, handler as () => void)
    }) as never
  })

  afterEach(() => {
    vi.useRealTimers()
    Object.defineProperty(window, 'innerWidth', {
      value: originalInnerWidth,
      writable: true,
      configurable: true,
    })
    window.getComputedStyle = originalGetComputedStyle
    window.addEventListener = originalAddEventListener
  })

  it('sets root font size based on screen width', () => {
    setRem()
    expect(document.documentElement.style.fontSize).toBe('100px')

    window.innerWidth = 960
    setRem()
    expect(document.documentElement.style.fontSize).toBe('50px')
  })

  it('initializes responsive behavior and updates on debounced resize', () => {
    initResponsive()

    expect(window.addEventListener).toHaveBeenCalledWith('resize', expect.any(Function))
    expect(document.documentElement.style.fontSize).toBe('100px')

    window.innerWidth = 960
    const resizeHandler = listeners.get('resize')
    resizeHandler?.()
    vi.advanceTimersByTime(100)

    expect(document.documentElement.style.fontSize).toBe('50px')
  })

  it('converts px values to responsive px', () => {
    expect(pxToResponsive(100)).toBe(100)
    window.innerWidth = 960
    expect(pxToResponsive(100)).toBe(50)
    window.innerWidth = 480
    expect(pxToResponsive(100)).toBe(25)
  })

  it('converts px values to rem based on the current scaled root size', () => {
    expect(pxToRem(100)).toBe('1rem')
    window.innerWidth = 960
    expect(pxToRem(100)).toBe('2rem')
    expect(pxToRem(0)).toBe('0rem')
  })

  it('returns the current scale and root font size', () => {
    expect(getCurrentScale()).toBe(1)
    window.innerWidth = 480
    expect(getCurrentScale()).toBe(0.25)

    document.documentElement.style.fontSize = '80px'
    expect(getCurrentFontSize()).toBe(80)
    window.getComputedStyle = vi.fn(() => ({ fontSize: 'invalid' })) as never
    expect(getCurrentFontSize()).toBe(100)
  })
})

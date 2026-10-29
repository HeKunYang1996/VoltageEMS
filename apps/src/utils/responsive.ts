// Responsive layout utilities

const DESIGN_WIDTH = 1920
const BASE_FONT_SIZE = 100 // 100px = 1rem

function getResponsiveScale(): number {
  // Scale from design width (1920px). Matches login/background layout expectations on wide viewports.
  const width = document.documentElement.clientWidth || window.innerWidth
  return width / DESIGN_WIDTH
}

/**
 * Set the root font size for rem-based responsive layouts.
 */
export function setRem(): void {
  const html = document.documentElement
  const fontSize = BASE_FONT_SIZE * getResponsiveScale()
  html.style.fontSize = `${fontSize}px`
}

let remResizeTimer: ReturnType<typeof setTimeout> | undefined

function scheduleRemUpdate(): void {
  clearTimeout(remResizeTimer)
  remResizeTimer = setTimeout(setRem, 100)
}

/**
 * Initialize responsive behavior.
 */
export function initResponsive(): void {
  setRem()

  window.addEventListener('resize', scheduleRemUpdate)
  window.addEventListener('orientationchange', () => {
    setRem()
  })
}

/**
 * Convert a design-time px value into a px value under the current viewport scale.
 */
export function pxToResponsive(designPx: number): number {
  return designPx * getResponsiveScale()
}

/**
 * Convert a design-time px value into rem.
 */
export function pxToRem(designPx: number): string {
  const currentBaseFontSize = BASE_FONT_SIZE * getResponsiveScale()
  const remValue = designPx / currentBaseFontSize
  return `${remValue}rem`
}

/**
 * Get the current viewport scale relative to the design size.
 */
export function getCurrentScale(): number {
  return getResponsiveScale()
}

/**
 * Get the current root font size in px.
 */
export function getCurrentFontSize(): number {
  const html = document.documentElement
  const fontSize = parseFloat(getComputedStyle(html).fontSize)
  return fontSize || BASE_FONT_SIZE
}

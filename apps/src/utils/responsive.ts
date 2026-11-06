const DESIGN_WIDTH = 1920
const BASE_FONT_SIZE = 100

export function setRem(): void {
  const html = document.documentElement
  const screenWidth = window.innerWidth
  const fontSize = BASE_FONT_SIZE * (screenWidth / DESIGN_WIDTH)

  html.style.fontSize = `${fontSize}px`
}

export function initResponsive(): void {
  setRem()

  window.addEventListener('resize', () => {
    clearTimeout((window as any).remResizeTimer)
    ;(window as any).remResizeTimer = setTimeout(setRem, 100)
  })
}

export function pxToResponsive(designPx: number): number {
  const screenWidth = window.innerWidth
  return designPx * (screenWidth / DESIGN_WIDTH)
}

export function pxToRem(designPx: number): string {
  const screenWidth = window.innerWidth
  const currentBaseFontSize = BASE_FONT_SIZE * (screenWidth / DESIGN_WIDTH)
  return `${designPx / currentBaseFontSize}rem`
}

export function getCurrentScale(): number {
  const screenWidth = window.innerWidth
  return screenWidth / DESIGN_WIDTH
}

export function getCurrentFontSize(): number {
  const html = document.documentElement
  const fontSize = parseFloat(getComputedStyle(html).fontSize)
  return fontSize || BASE_FONT_SIZE
}

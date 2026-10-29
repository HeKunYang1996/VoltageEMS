/**
 * Runs before the main app bundle so rem is stable on first paint (avoids layout jump).
 */
import { initResponsive } from './utils/responsive'

initResponsive()

/**
 * Poster service for the mobile auth fan shell.
 *
 * There is no backend random-covers endpoint (only /search which requires a
 * query term). We therefore generate procedural-gradient placeholders — the
 * same approach used by AuthPosterCollage.vue on desktop.
 *
 * Session-level cache (sessionStorage key: auth-fan-posters-v1) persists the
 * chosen hue indices so the fan looks stable across login ↔ register
 * navigations within the same session.
 */

export interface PosterRef {
  /** CSS background shorthand (gradient). */
  background: string
  /** Decorative glyph displayed on top of the gradient. */
  glyph: string
}

const CACHE_KEY = 'auth-fan-posters-v1'

/** Three hue anchors for the procedural fallbacks (coral, matcha, sakura). */
const FALLBACK: readonly number[] = [28, 145, 350]

const PALETTES: ReadonlyArray<readonly [string, string, string]> = [
  ['#1a1130', '#46225a', '#d96b6b'],
  ['#0f1f2e', '#1d4256', '#7fc6c1'],
  ['#2a1024', '#6b1f4d', '#f0a3a3'],
  ['#1a2a18', '#3b6a3a', '#dceb7b'],
  ['#241a30', '#4d3a72', '#b89bff'],
  ['#2c1612', '#82382a', '#f4c87a'],
  ['#0e1726', '#27407a', '#9bb6ff'],
  ['#2c1622', '#7c2a55', '#ffb9d4'],
  ['#1a1f0e', '#4a5523', '#f7e36a'],
  ['#1f1023', '#5a2470', '#ff8ab8'],
] as const

const GLYPHS = ['◐', '✦', '▲', '◇', '◍', '✿', '☄', '✺', '◉', '▽'] as const

function buildGradient(seed: number): string {
  const p = PALETTES[seed % PALETTES.length]!
  const angle = (seed * 37) % 360
  const cx = 20 + ((seed * 7) % 60)
  const cy = 10 + ((seed * 11) % 40)
  return `radial-gradient(120% 80% at ${cx}% ${cy}%, ${p[2]}55, transparent 60%), linear-gradient(${angle}deg, ${p[0]}, ${p[1]} 60%, ${p[2]}aa 130%)`
}

function makeFallbacks(n: number): PosterRef[] {
  return Array.from({ length: n }, (_, i) => {
    const seed = FALLBACK[i % FALLBACK.length]!
    return {
      background: buildGradient(seed),
      glyph: GLYPHS[seed % GLYPHS.length]!,
    }
  })
}

/**
 * Returns n poster references for the mobile auth fan.
 *
 * Results are cached in sessionStorage for the lifetime of the tab so the
 * fan stays visually stable across page navigations.
 *
 * Note: This function always resolves — it never throws. AniList cover
 * fetching is intentionally omitted because no backend random-covers endpoint
 * exists; adding one would expand scope into the server crate (out of scope
 * for Track 3). The procedural fallback is the canonical implementation.
 */
export async function getRandomCoverPosters(n = 3): Promise<PosterRef[]> {
  try {
    const cached = sessionStorage.getItem(CACHE_KEY)
    if (cached) {
      const parsed: PosterRef[] = JSON.parse(cached)
      if (Array.isArray(parsed) && parsed.length >= n) {
        return parsed.slice(0, n)
      }
    }
  } catch {
    // sessionStorage unavailable or JSON malformed — fall through
  }

  const posters = makeFallbacks(n)

  try {
    sessionStorage.setItem(CACHE_KEY, JSON.stringify(posters))
  } catch {
    // quota exceeded — continue without caching
  }

  return posters
}

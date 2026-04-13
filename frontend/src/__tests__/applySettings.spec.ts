import { describe, it, expect, beforeEach, vi } from 'vitest'

// Mock the i18n plugin before importing applySettings, since it runs module-level code
vi.mock('@/plugins/i18n', () => ({
  i18n: {
    global: {
      locale: { value: 'en' }
    }
  }
}))

import { applySettings } from '@/services/applySettings'

describe('applySettings', () => {
  beforeEach(() => {
    // Reset data-theme attribute between tests
    document.documentElement.removeAttribute('data-theme')
  })

  it('sets data-theme to light on the html element', async () => {
    await applySettings({
      theme_preference: 'light',
      language_preference: 'en',
      title_language_preference: 'English',
      timezone: 'UTC'
    })
    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
  })

  it('sets data-theme to dark on the html element', async () => {
    await applySettings({
      theme_preference: 'dark',
      language_preference: 'en',
      title_language_preference: 'English',
      timezone: 'UTC'
    })
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })

  it('updates the i18n locale to match language_preference', async () => {
    const { i18n } = await import('@/plugins/i18n')
    await applySettings({
      theme_preference: 'dark',
      language_preference: 'pt',
      title_language_preference: 'English',
      timezone: 'UTC'
    })
    expect(i18n.global.locale.value).toBe('pt')
  })

  it('replaces any previously set data-theme attribute', async () => {
    document.documentElement.setAttribute('data-theme', 'light')
    await applySettings({
      theme_preference: 'dark',
      language_preference: 'en',
      title_language_preference: 'English',
      timezone: 'UTC'
    })
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
  })
})

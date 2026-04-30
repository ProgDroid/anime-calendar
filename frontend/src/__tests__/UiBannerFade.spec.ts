import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiBannerFade from '@/components/ui/UiBannerFade.vue'

const VERBATIM_MASK = 'linear-gradient(to right, rgba(0,0,0,0) 0%, rgba(0,0,0,0) 50%, rgba(0,0,0,0.2) 65%, rgba(0,0,0,1) 95%)'

describe('UiBannerFade', () => {
  it('rendered HTML contains the verbatim mask string', () => {
    const w = mount(UiBannerFade, { props: { selected: true, posterUrl: '/p.png' } })
    expect(w.html()).toContain(VERBATIM_MASK)
  })

  it('selected=false sets opacity-0 class', () => {
    const w = mount(UiBannerFade, { props: { selected: false, posterUrl: '/p.png' } })
    expect(w.find('[data-testid="banner"]').classes()).toContain('opacity-0')
  })

  it('selected=true sets opacity-100 class', () => {
    const w = mount(UiBannerFade, { props: { selected: true, posterUrl: '/p.png' } })
    expect(w.find('[data-testid="banner"]').classes()).toContain('opacity-100')
  })

  it('posterUrl=null falls back to flat accent-soft fill', () => {
    const w = mount(UiBannerFade, { props: { selected: true, posterUrl: null } })
    const style = w.find('[data-testid="banner"]').attributes('style') ?? ''
    expect(style).toContain('var(--accent-1-soft)')
  })
})

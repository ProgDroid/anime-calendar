import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import PosterCollage from '../PosterCollage.vue';

describe('PosterCollage', () => {
  it('renders up to 4 posters in a 2x2 grid', () => {
    const wrapper = mount(PosterCollage, {
      props: {
        urls: ['a.jpg', 'b.jpg', 'c.jpg', 'd.jpg', 'e.jpg'],
      },
    });
    const imgs = wrapper.findAll('[data-testid="poster-tile"]');
    expect(imgs).toHaveLength(4);
  });

  it('fills empty slots when fewer than 4 urls', () => {
    const wrapper = mount(PosterCollage, { props: { urls: ['a.jpg'] } });
    expect(wrapper.findAll('[data-testid="poster-tile"]')).toHaveLength(1);
    expect(wrapper.findAll('[data-testid="poster-empty"]')).toHaveLength(3);
  });

  it('renders fully empty when no urls', () => {
    const wrapper = mount(PosterCollage, { props: { urls: [] } });
    expect(wrapper.findAll('[data-testid="poster-empty"]')).toHaveLength(4);
  });
});

<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useViewportLayout } from '@/composables/useViewportLayout'

defineOptions({ name: 'AppFooter' })

const { t } = useI18n()
const { isMobile } = useViewportLayout()
const route = useRoute()

const bottomBarVisible = computed(() => isMobile.value && route.meta.bottomTabBar !== false)
const year = new Date().getFullYear()
</script>

<template>
  <footer
    :class="['border-t border-line bg-bg-0', bottomBarVisible ? 'pb-tab-bar' : '']"
    :aria-label="t('app.nav.footer')"
  >
    <div
      class="container mx-auto px-4 py-4 flex flex-wrap items-center justify-between gap-3 text-xs text-fg-3"
    >
      <span>{{ t('footer.copyright', { year }) }}</span>
      <nav class="flex items-center gap-4" :aria-label="t('footer.linksLabel')">
        <RouterLink to="/privacy" class="hover:text-fg-1 transition-colors">
          {{ t('footer.privacy') }}
        </RouterLink>
        <RouterLink to="/terms" class="hover:text-fg-1 transition-colors">
          {{ t('footer.terms') }}
        </RouterLink>
        <a
          href="https://anilist.co"
          target="_blank"
          rel="noopener noreferrer"
          class="hover:text-fg-1 transition-colors"
        >
          {{ t('footer.poweredBy') }}
        </a>
      </nav>
    </div>
  </footer>
</template>

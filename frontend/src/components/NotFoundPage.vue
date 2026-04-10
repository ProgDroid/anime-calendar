<template>
  <div class="min-h-screen bg-base-200 flex items-center justify-center">
    <div class="text-center">
      <h1 class="text-6xl font-bold text-base-content">404</h1>
      <p class="text-xl mt-4 text-base-content/70">{{ $t('errors.notFound') }}</p>
      <router-link to="/my-calendars" class="btn btn-primary mt-6">
        {{ $t('app.home') }}
      </router-link>
    </div>
  </div>
</template>

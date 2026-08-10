<script setup lang="ts">
const REPO = "https://github.com/CollinFiske/stash";
</script>

<template>
  <div class="page-container">
    <h1 class="page-h1">Contact</h1>

    <p class="page-desc">
      Found a bug or have an idea for a feature? Everything happens in the open on GitHub.
    </p>

    <div class="contact-card">
      <div class="contact-item">
        <span class="contact-icon">🐛</span>
        <div>
          <div class="contact-label">Report a bug</div>
          <a class="contact-value" :href="`${REPO}/issues/new`" target="_blank" rel="noopener noreferrer">
            Open an issue
          </a>
        </div>
      </div>

      <div class="contact-item">
        <span class="contact-icon">💡</span>
        <div>
          <div class="contact-label">Request a feature</div>
          <a class="contact-value" :href="`${REPO}/discussions`" target="_blank" rel="noopener noreferrer">
            Start a discussion
          </a>
        </div>
      </div>

      <div class="contact-item">
        <span class="contact-icon">⌥</span>
        <div>
          <div class="contact-label">Source code</div>
          <a class="contact-value" :href="REPO" target="_blank" rel="noopener noreferrer">
            CollinFiske/stash
          </a>
        </div>
      </div>
    </div>

    <p class="page-muted">
      Filing an issue is the fastest way to get a fix — it keeps the report, the
      discussion and the eventual change in one place.<br /><br />Thank you!
    </p>
  </div>
</template>

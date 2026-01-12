<template>
  <div class="toastHost">
    <div v-for="t in toasts.items" :key="t.id" class="toast" :class="t.kind">
      <div class="t">{{ t.title }}</div>
      <div class="m">{{ t.message }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { toasts } from "../app";
</script>

<style scoped>
.toastHost{position: fixed; right: 14px; top: 70px; z-index: 60; display:flex; flex-direction:column; gap:10px}
.toast{
  width: 320px;
  padding: 10px 12px;
  border-radius: 14px;
  border:1px solid var(--line);
  background: rgba(15,22,33,.9);
  box-shadow: var(--shadow);
}
.toast .t{font-weight:700; margin-bottom:6px}
.toast .m{color:var(--muted); line-height:1.45}
.toast.ok{border-color: rgba(52,211,153,.35)}
.toast.bad{border-color: rgba(251,113,133,.35)}
.toast.warn{border-color: rgba(251,191,36,.35)}
</style>

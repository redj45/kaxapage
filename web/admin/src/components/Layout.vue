<template>
    <div class="app">
        <header class="topbar">
            <a class="brand" href="/" target="_blank" rel="noreferrer">
                <div class="logo">KP</div>
                <div class="meta">
                    <div class="name">KaxaPage</div>
                    <div class="tag">Admin</div>
                </div>
            </a>

            <nav class="nav">
                <RouterLink to="/setup">Setup</RouterLink>
                <RouterLink to="/services">Services</RouterLink>
                <RouterLink to="/incidents">Incidents</RouterLink>
                <RouterLink to="/public">Public</RouterLink>
                <RouterLink to="/settings">Settings</RouterLink>
            </nav>

            <div class="right">
                <select
                    v-model="selectedPageId"
                    class="select"
                    :disabled="pages.length === 0"
                    @change="onChangePage"
                >
                    <option v-if="pages.length === 0" value="">No pages</option>
                    <option v-for="p in pages" :key="p.id" :value="p.id">
                        {{ p.title }}
                    </option>
                </select>
                <span class="pill" :class="connOk ? 'ok' : 'bad'">{{
                    connText
                }}</span>
            </div>
        </header>

        <main class="container">
            <ToastHost />
            <RouterView />
        </main>
    </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import ToastHost from "./ToastHost.vue";
import { api, getPageId, setPageId, setPageSlug } from "../api";

type PageItem = { id: string; slug: string; title: string; published: boolean };

const pages = ref<PageItem[]>([]);
const selectedPageId = ref(getPageId());
const connOk = ref(true);
const connText = ref("...");

async function refreshPages() {
    const r = await api.pages();
    if (r.status === 401) {
        connOk.value = false;
        connText.value = document.cookie.includes("kp_admin=")
            ? "unauthorized"
            : "no token";
        pages.value = [];
        return;
    }
    if (r.body && "ok" in r.body && r.body.ok) {
        pages.value = r.body.data;
        // auto-select
        const chosen =
            pages.value.find((p) => p.id === selectedPageId.value) ||
            pages.value[0];
        if (chosen) {
            selectedPageId.value = chosen.id;
            setPageId(chosen.id);
            setPageSlug(chosen.slug);
        }
    }
}

function onChangePage() {
    const p = pages.value.find((x) => x.id === selectedPageId.value);
    if (!p) return;
    setPageId(p.id);
    setPageSlug(p.slug);
}

async function checkHealth() {
    try {
        const res = await fetch("/healthz");
        connOk.value = res.ok;
        connText.value = res.ok ? "connected" : "db down";
    } catch {
        connOk.value = false;
        connText.value = "offline";
    }
}

onMounted(async () => {
    await checkHealth();
    await refreshPages();
});
</script>

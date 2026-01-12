<template>
    <div class="card">
        <div class="h1">Public</div>
        <p class="p">Links and preview JSON response.</p>

        <div class="hr"></div>

        <div class="row">
            <div class="card" style="padding: 12px">
                <div class="h2">Status Page</div>
                <div class="p">{{ pageUrl }}</div>
                <a
                    class="btn small"
                    :href="pageUrl"
                    target="_blank"
                    rel="noreferrer"
                    >Open</a
                >
            </div>

            <div class="card" style="padding: 12px">
                <div class="h2">Public JSON</div>
                <div class="p">{{ apiUrl }}</div>
                <a
                    class="btn small secondary"
                    :href="apiUrl"
                    target="_blank"
                    rel="noreferrer"
                    >Open</a
                >
            </div>
        </div>

        <div class="hr"></div>

        <div class="h2">Preview</div>
        <pre class="card pre">{{ pretty }}</pre>
    </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api, getPageSlug } from "../api";
import { pushToast } from "../app";
import { useRouter } from "vue-router";

const router = useRouter();
const data = ref<any>(null);

const slug = computed(() => getPageSlug());
const apiUrl = computed(
    () => `${location.origin}/api/v1/public/pages/${slug.value}`,
);
const pageUrl = computed(() => `${location.origin}/`);

const pretty = computed(() => JSON.stringify(data.value, null, 2));

onMounted(async () => {
    if (!slug.value) {
        pushToast("warn", "No slug", "Go to Setup or select a page.");
        router.push("/setup");
        return;
    }
    const r = await api.publicPage(slug.value);
    if (!r.body || r.body.ok === false) {
        pushToast("bad", "Failed", r.body?.error?.message || "unknown error");
        return;
    }
    data.value = r.body.data;
});
</script>

<style scoped>
.pre {
    padding: 12px;
    overflow: auto;
    white-space: pre;
    font-family: var(--mono);
    font-size: 12px;
}
</style>

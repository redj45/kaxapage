<template>
  <div class="card">
    <div class="h1">Initial Setup</div>
    <p class="p">Run this once to create workspace, status page, and initial services.</p>

    <div class="hr"></div>

    <div class="row">
      <div>
        <div class="h2">Workspace</div>
        <input class="input" v-model="workspaceName" placeholder="Workspace name" />
      </div>
      <div>
        <div class="h2">Page slug</div>
        <input class="input" v-model="pageSlug" placeholder="status" />
      </div>
      <div>
        <div class="h2">Page title</div>
        <input class="input" v-model="pageTitle" placeholder="Acme Status" />
      </div>
    </div>

    <div class="hr"></div>

    <div class="h2">Initial services</div>
    <div class="row">
      <input class="input" v-model="svc1" placeholder="Service 1" />
      <input class="input" v-model="svc2" placeholder="Service 2" />
    </div>

    <div class="hr"></div>

    <button class="btn" :disabled="busy" @click="doBootstrap">
      {{ busy ? "Working..." : "Create workspace & page" }}
    </button>

    <div class="hr"></div>

    <div class="p">
      If you already bootstrapped, go to <RouterLink to="/services" class="link">Services</RouterLink>.
      Make sure you are <RouterLink to="/settings" class="link">logged in</RouterLink> first.
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { api, setPageId, setPageSlug } from "../api";
import { pushToast } from "../app";
import { useRouter } from "vue-router";

const router = useRouter();

const workspaceName = ref("Acme");
const pageSlug = ref("status");
const pageTitle = ref("Acme Status");

const svc1 = ref("API");
const svc2 = ref("Dashboard");

const busy = ref(false);

async function doBootstrap() {
  busy.value = true;
  try {
    const services = [svc1.value.trim(), svc2.value.trim()]
      .filter(Boolean)
      .map(name => ({ name }));

    const r = await api.bootstrap({
      workspace_name: workspaceName.value.trim(),
      page: { slug: pageSlug.value.trim(), title: pageTitle.value.trim() },
      services,
    });

    if (r.status === 401) {
      pushToast("bad", "Unauthorized", "Log in first via Settings.");
      router.push("/settings");
      return;
    }
    if (!r.body || !("ok" in r.body) || r.body.ok === false) {
      pushToast("bad", "Bootstrap failed", r.body?.error?.message || "unknown error");
      return;
    }

    setPageId(r.body.data.status_page_id);
    setPageSlug(r.body.data.page_slug);

    pushToast("ok", "Bootstrapped", "Workspace + page created.");
    router.push("/services");
  } finally {
    busy.value = false;
  }
}

onMounted(async () => {
  const r = await api.pages();
  if (r.status === 200 && r.body && "ok" in r.body && r.body.ok && r.body.data.length) {
    pushToast("ok", "Already set up", "Status page exists. You can manage services.");
  }
});
</script>

<style scoped>
.link{color: var(--accent)}
</style>

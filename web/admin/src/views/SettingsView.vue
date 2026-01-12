<template>
  <div class="card">
    <div class="h1">Settings</div>
    <p class="p">Token is stored in an httpOnly cookie — not readable by JavaScript.</p>

    <div class="hr"></div>

    <div class="h2">Admin token</div>
    <div class="row" style="gap:10px; align-items:flex-end">
      <input class="input" v-model="token" type="password" placeholder="ADMIN_TOKEN" />
      <button class="btn" :disabled="busy" @click="login">Log in</button>
    </div>

    <div class="hr"></div>

    <div class="h2">Current page</div>
    <div class="row">
      <div>
        <div class="p">status_page_id</div>
        <input class="input" v-model="pageId" placeholder="uuid" />
      </div>
      <div>
        <div class="p">slug</div>
        <input class="input" v-model="pageSlug" placeholder="status" />
      </div>
    </div>
    <button class="btn" style="margin-top:10px" @click="savePage">Save page settings</button>

    <div class="hr"></div>

    <button class="btn secondary" @click="logout">Log out</button>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { api, getPageId, setPageId, getPageSlug, setPageSlug } from "../api";
import { pushToast } from "../app";
import { useRouter } from "vue-router";

const router = useRouter();

const token = ref("");
const pageId = ref(getPageId());
const pageSlug = ref(getPageSlug());
const busy = ref(false);

async function login() {
  if (!token.value.trim()) {
    pushToast("warn", "Empty token", "Enter the ADMIN_TOKEN.");
    return;
  }
  busy.value = true;
  try {
    const r = await api.login(token.value.trim());
    if (r.status === 200) {
      token.value = "";
      pushToast("ok", "Logged in", "Session cookie set.");
    } else {
      pushToast("bad", "Unauthorized", "Wrong token.");
    }
  } finally {
    busy.value = false;
  }
}

function savePage() {
  setPageId(pageId.value.trim());
  setPageSlug(pageSlug.value.trim());
  pushToast("ok", "Saved", "Page settings saved.");
}

async function logout() {
  await api.logout();
  pushToast("warn", "Logged out", "Session cookie cleared.");
  router.push("/setup");
}
</script>

<template>
  <div class="card" v-if="data">
    <div class="row" style="align-items:center; justify-content:space-between">
      <div>
        <div class="h1">{{ data.incident.title }}</div>
        <div class="row" style="gap:10px; margin-top:8px">
          <span class="pill">{{ data.incident.status }}</span>
          <span class="pill">Impact: {{ data.incident.impact }}</span>
          <span class="pill">Started: {{ fmt(data.incident.started_at) }}</span>
        </div>
      </div>
      <RouterLink class="btn secondary" to="/incidents">Back</RouterLink>
    </div>

    <div class="hr"></div>

    <div class="h2">Updates</div>
    <div v-for="u in data.updates" :key="u.id" class="card" style="margin-top:10px; padding:12px">
      <div class="p" style="margin:0 0 6px; font-family:var(--mono)">{{ fmt(u.created_at) }}</div>
      <div>{{ u.message }}</div>
    </div>

    <div class="hr"></div>

    <div class="h2">Add update</div>
    <div class="row">
      <div>
        <div class="p">Status (optional)</div>
        <select class="select" v-model="updStatus">
          <option value="">(keep)</option>
          <option value="investigating">investigating</option>
          <option value="identified">identified</option>
          <option value="monitoring">monitoring</option>
        </select>
      </div>
      <div>
        <div class="p">Tip</div>
        <div class="p">Leave empty to keep current.</div>
      </div>
    </div>

    <div style="margin-top:10px">
      <textarea class="textarea" v-model="updMsg" placeholder="Update message..."></textarea>
    </div>
    <div style="margin-top:10px">
      <button class="btn" @click="postUpdate">Post update</button>
    </div>

    <div class="hr"></div>

    <div class="h2">Resolve</div>
    <p class="p">This sets status to resolved and adds a final update.</p>
    <textarea class="textarea" v-model="resMsg" placeholder="Resolution message..."></textarea>
    <div style="margin-top:10px">
      <button class="btn danger" @click="resolve">Resolve incident</button>
    </div>

    <template v-if="data.affected_services?.length">
      <div class="hr"></div>
      <div class="h2">Affected services</div>
      <div class="row" style="gap:6px; flex-wrap:wrap; margin-top:6px">
        <span class="pill" v-for="s in data.affected_services" :key="s.id">{{ s.name }}</span>
      </div>
    </template>
  </div>

  <div class="card" v-else>
    <div class="h1">Loading...</div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { api } from "../api";
import { pushToast } from "../app";

const route = useRoute();
const router = useRouter();
const id = String(route.params.id || "");

const data = ref<any | null>(null);

const updMsg = ref("");
const updStatus = ref("");
const resMsg = ref("");

function fmt(s: string) {
  try { return new Date(s).toLocaleString(); } catch { return s; }
}

async function load() {
  const r = await api.getIncident(id);
  if (r.status === 401) {
    pushToast("bad", "Unauthorized", "Log in via Settings.");
    router.push("/settings");
    return;
  }
  if (!r.body?.ok) {
    pushToast("bad", "Load failed", r.body?.error?.message || "unknown error");
    router.push("/incidents");
    return;
  }
  data.value = r.body.data;
}

async function postUpdate() {
  const payload: any = { message: updMsg.value.trim() };
  if (updStatus.value) payload.status = updStatus.value;

  const r = await api.addUpdate(id, payload);
  if (!r.body?.ok) {
    pushToast("bad", "Add update failed", r.body?.error?.message || "unknown error");
    return;
  }
  pushToast("ok", "Added", "Update posted.");
  updMsg.value = "";
  load();
}

async function resolve() {
  if (!confirm("Mark this incident as resolved? This cannot be undone.")) return;
  const r = await api.resolveIncident(id, { message: resMsg.value.trim() });
  if (!r.body?.ok) {
    pushToast("bad", "Resolve failed", r.body?.error?.message || "unknown error");
    return;
  }
  pushToast("ok", "Resolved", "Incident marked as resolved.");
  load();
}

onMounted(load);
</script>

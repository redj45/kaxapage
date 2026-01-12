<template>
  <div class="card">
    <div class="h1">Incidents</div>
    <p class="p">Create, update, and resolve incidents.</p>

    <div class="hr"></div>

    <div class="h2">Create new incident</div>
    <div class="row">
      <div>
        <div class="p">Title</div>
        <input class="input" v-model="title" placeholder="Incident title" />
      </div>
      <div>
        <div class="p">Impact</div>
        <select class="select" v-model="impact">
          <option v-for="v in impacts" :key="v" :value="v">{{ v }}</option>
        </select>
      </div>
    </div>

    <div style="margin-top:10px">
      <textarea class="textarea" v-model="message" placeholder="Initial update message"></textarea>
    </div>

    <div v-if="services.length" style="margin-top:10px">
      <div class="p">Affected services (optional)</div>
      <div class="row" style="gap:12px; flex-wrap:wrap; margin-top:4px">
        <label v-for="s in services" :key="s.id" style="display:flex; align-items:center; gap:5px; cursor:pointer">
          <input type="checkbox" :value="s.id" v-model="affectedServices" />
          {{ s.name }}
        </label>
      </div>
    </div>

    <div style="margin-top:10px">
      <button class="btn" @click="create">Create incident</button>
      <button class="btn secondary" style="margin-left:10px" @click="load">Reload</button>
    </div>

    <div class="hr"></div>

    <div class="h2">Recent incidents</div>
    <table class="table" v-if="items.length">
      <thead>
        <tr>
          <th>Title</th>
          <th>Status</th>
          <th>Impact</th>
          <th>Started</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="it in items" :key="it.id">
          <td>{{ it.title }}</td>
          <td>{{ it.status }}</td>
          <td>{{ it.impact }}</td>
          <td class="p">{{ fmt(it.started_at) }}</td>
          <td><RouterLink class="btn small" :to="`/incident/${it.id}`">Open</RouterLink></td>
        </tr>
      </tbody>
    </table>
    <div v-else class="p">No incidents yet.</div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api, getPageId, IncidentImpact, ServiceItem } from "../api";
import { pushToast } from "../app";
import { useRouter } from "vue-router";

const router = useRouter();
const impacts: IncidentImpact[] = ["minor", "major", "critical", "none"];

const title = ref("");
const impact = ref<IncidentImpact>("minor");
const message = ref("");
const services = ref<ServiceItem[]>([]);
const affectedServices = ref<string[]>([]); // selected service IDs

const items = ref<any[]>([]);

function fmt(s: string) {
  try { return new Date(s).toLocaleString(); } catch { return s; }
}

async function load() {
  const pid = getPageId();
  if (!pid) {
    pushToast("warn", "No page selected", "Go to Setup or choose a page.");
    router.push("/setup");
    return;
  }

  const sr = await api.services();
  if (sr.body?.ok) services.value = sr.body.data;

  const r = await api.incidents(pid, 30);
  if (r.status === 401) {
    pushToast("bad", "Unauthorized", "Log in via Settings.");
    router.push("/settings");
    return;
  }
  if (!r.body?.ok) {
    pushToast("bad", "Load failed", r.body?.error?.message || "unknown error");
    return;
  }
  items.value = r.body.data.items || [];
}

async function create() {
  const pid = getPageId();
  if (!pid) return;

  const r = await api.createIncident({
    status_page_id: pid,
    title: title.value.trim(),
    impact: impact.value,
    status: "investigating",
    started_at: null,
    message: message.value.trim(),
    service_ids: affectedServices.value.length ? affectedServices.value : null,
  });

  if (!r.body?.ok) {
    pushToast("bad", "Create failed", r.body?.error?.message || "unknown error");
    return;
  }
  pushToast("ok", "Created", "Incident created.");
  title.value = ""; message.value = ""; affectedServices.value = [];
  load();
}

onMounted(load);
</script>

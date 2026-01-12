<template>
  <div class="card">
    <div class="h1">Services</div>
    <p class="p">Manage service statuses and ordering.</p>

    <div class="hr"></div>

    <table class="table" v-if="items.length">
      <thead>
        <tr>
          <th>Name</th>
          <th>Status</th>
          <th>Position</th>
          <th>Updated</th>
          <th>Actions</th>
        </tr>
      </thead>

      <tbody>
        <tr v-for="s in items" :key="s.id">
          <td>
            <input class="input" v-model="s._name" />
            <div style="margin-top:8px">
              <input class="input" v-model="s._desc" placeholder="Description" />
            </div>
          </td>

          <td>
            <select class="select" v-model="s._status">
              <option v-for="v in statuses" :key="v" :value="v">{{ v }}</option>
            </select>
          </td>

          <td>
            <input class="input" type="number" v-model.number="s._pos" />
          </td>

          <td class="p">{{ fmt(s.updated_at) }}</td>

          <td>
            <div class="row" style="gap:8px">
              <button class="btn small" @click="save(s)">Save</button>
              <button class="btn small danger" @click="del(s)">Delete</button>
            </div>
          </td>
        </tr>
      </tbody>
    </table>

    <div v-else class="p">No services yet.</div>

    <div class="hr"></div>

    <div class="h2">Add new service</div>
    <div class="row">
      <input class="input" v-model="newName" placeholder="Name" />
      <select class="select" v-model="newStatus">
        <option v-for="v in statuses" :key="v" :value="v">{{ v }}</option>
      </select>
      <input class="input" type="number" v-model.number="newPos" placeholder="Position" />
    </div>
    <div style="margin-top:10px">
      <input class="input" v-model="newDesc" placeholder="Description (optional)" />
    </div>
    <div style="margin-top:10px">
      <button class="btn" @click="create">Add service</button>
      <button class="btn secondary" style="margin-left:10px" @click="load">Reload</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { api, ServiceItem, ServiceStatus } from "../api";
import { pushToast } from "../app";
import { useRouter } from "vue-router";

const router = useRouter();

const statuses: ServiceStatus[] = ["operational", "maintenance", "degraded", "partial_outage", "major_outage"];

type UiService = ServiceItem & {
  _name: string;
  _desc: string;
  _status: ServiceStatus;
  _pos: number;
};

const items = ref<UiService[]>([]);

const newName = ref("");
const newDesc = ref("");
const newStatus = ref<ServiceStatus>("operational");
const newPos = ref(0);

function fmt(s: string) {
  try { return new Date(s).toLocaleString(); } catch { return s; }
}

async function load() {
  const r = await api.services();
  if (r.status === 401) {
    pushToast("bad", "Unauthorized", "Log in via Settings.");
    router.push("/settings");
    return;
  }
  if (!r.body || r.body.ok === false) {
    pushToast("bad", "Load failed", r.body?.error?.message || "unknown error");
    return;
  }
  items.value = r.body.data.map(s => ({
    ...s,
    _name: s.name,
    _desc: s.description || "",
    _status: s.status,
    _pos: s.position,
  }));
}

async function create() {
  const r = await api.createService({
    name: newName.value.trim(),
    description: newDesc.value.trim() || null,
    position: newPos.value,
    status: newStatus.value,
  });
  if (!r.body || r.body.ok === false) {
    pushToast("bad", "Create failed", r.body?.error?.message || "unknown error");
    return;
  }
  pushToast("ok", "Created", "Service added.");
  newName.value = ""; newDesc.value = ""; newPos.value = 0; newStatus.value = "operational";
  load();
}

async function save(s: UiService) {
  const r = await api.patchService(s.id, {
    name: s._name.trim(),
    description: s._desc.trim() || null,
    position: s._pos,
    status: s._status,
  });
  if (!r.body || r.body.ok === false) {
    pushToast("bad", "Save failed", r.body?.error?.message || "unknown error");
    return;
  }
  pushToast("ok", "Saved", "Service updated.");
  load();
}

async function del(s: UiService) {
  if (!confirm(`Delete "${s.name}"?`)) return;
  const r = await api.deleteService(s.id);
  if (r.status !== 204) {
    pushToast("bad", "Delete failed", r.body?.error?.message || "unknown error");
    return;
  }
  pushToast("ok", "Deleted", "Service removed.");
  load();
}

onMounted(load);
</script>

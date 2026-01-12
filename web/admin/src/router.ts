import { createRouter, createWebHashHistory } from "vue-router";

import SetupView from "./views/SetupView.vue";
import ServicesView from "./views/ServicesView.vue";
import IncidentsView from "./views/IncidentsView.vue";
import IncidentView from "./views/IncidentView.vue";
import PublicView from "./views/PublicView.vue";
import SettingsView from "./views/SettingsView.vue";

export const router = createRouter({
  history: createWebHashHistory(),
  linkActiveClass: "active",
  linkExactActiveClass: "active",
  routes: [
    { path: "/", redirect: "/services" },
    { path: "/setup", component: SetupView },
    { path: "/services", component: ServicesView },
    { path: "/incidents", component: IncidentsView },
    { path: "/incident/:id", component: IncidentView, props: true },
    { path: "/public", component: PublicView },
    { path: "/settings", component: SettingsView },
  ],
});

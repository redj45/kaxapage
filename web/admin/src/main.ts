import "./style.css";

import { createApp } from "vue";
import { router } from "./router";
import Layout from "./components/Layout.vue";

createApp(Layout).use(router).mount("#app");
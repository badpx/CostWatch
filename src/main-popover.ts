import "./styles/themes.css";
import { createApp } from "vue";
import Popover from "./popover/Popover.vue";
import i18n from "./i18n";
import { initLocale } from "./composables/useLocale";

const app = createApp(Popover);
app.use(i18n);
app.mount("#app");
initLocale();
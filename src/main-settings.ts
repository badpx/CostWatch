import "./styles/themes.css";
import { createApp } from "vue";
import Settings from "./settings/Settings.vue";
import i18n from "./i18n";
import { initLocale } from "./composables/useLocale";

initLocale().then(() => {
  const app = createApp(Settings);
  app.use(i18n);
  app.mount("#app");
});
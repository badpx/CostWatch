import { computed, ref, watch, type ComputedRef } from "vue";
import type { ProviderState } from "../types";

function getIconKey(icon: ProviderState["icon"]): string {
  if (typeof icon === "object" && "Builtin" in icon) return icon.Builtin;
  if (typeof icon === "object" && "Custom" in icon) return icon.Custom;
  return "";
}

const lobehubMap: Record<string, string> = {
  deepinfra: "deepinfra",
  runware: "runware",
};

export function getIconUrl(key: string): string {
  const name = lobehubMap[key] || key;
  return `https://unpkg.com/@lobehub/icons-static-svg/icons/${name}.svg`;
}

export function useProviderIcon(iconRaw: ComputedRef<ProviderState["icon"]>) {
  const iconKey = computed(() => getIconKey(iconRaw.value));
  const iconSrc = ref("");

  watch(
    iconKey,
    (key) => {
      iconSrc.value = getIconUrl(key);
    },
    { immediate: true }
  );

  function onIconError(event: Event) {
    const img = event.target as HTMLImageElement;
    const key = iconKey.value;
    if (img.src.startsWith("https://unpkg.com")) {
      img.src = `/icons/${key}.svg`;
    } else if (img.src.endsWith(".svg")) {
      img.src = img.src.replace(/\.svg$/, ".png");
    }
  }

  return { iconKey, iconSrc, onIconError };
}

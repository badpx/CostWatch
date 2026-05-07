<template>
  <div class="trend-chart" ref="containerRef">
    <canvas ref="canvasRef" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, nextTick } from "vue";
import type { HistoryPoint, ProviderState } from "../types";
import { rangeToMs, formatVal } from "../utils/chart";

const props = defineProps<{
  dataPoints: HistoryPoint[];
  range: string;
  currency: ProviderState["currency"];
}>();

const containerRef = ref<HTMLDivElement>();
const canvasRef = ref<HTMLCanvasElement>();

function draw() {
  const canvas = canvasRef.value;
  const container = containerRef.value;
  if (!canvas || !container) return;

  const dpr = window.devicePixelRatio || 1;
  const rect = container.getBoundingClientRect();
  const w = rect.width;
  const h = 52;

  canvas.width = w * dpr;
  canvas.height = h * dpr;
  canvas.style.width = `${w}px`;
  canvas.style.height = `${h}px`;

  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.scale(dpr, dpr);

  // Read theme-aware CSS variable for grid lines
  const cs = getComputedStyle(container);
  const gridColor = cs.getPropertyValue("--border").trim() || "rgba(255,255,255,0.06)";
  const labelColor = cs.getPropertyValue("--text-tertiary").trim() || "#666";

  const points = props.dataPoints;
  const padY = 10;
  const chartH = h - padY * 2;

  ctx.clearRect(0, 0, w, h);

  // Grid lines (always drawn, even as skeleton)
  ctx.strokeStyle = gridColor;
  ctx.lineWidth = 0.5;
  ctx.beginPath();
  ctx.moveTo(0, padY);
  ctx.lineTo(w, padY);
  ctx.moveTo(0, h - padY);
  ctx.lineTo(w, h - padY);
  ctx.stroke();

  // Skeleton mode: no data to draw
  if (points.length === 0) return;

  const values = points.map((p) => p.value);
  let minVal = Math.min(...values);
  let maxVal = Math.max(...values);

  // Round boundaries to integers for clean labels (e.g. 93.73 → 94 / 93)
  maxVal = Math.ceil(maxVal);
  minVal = Math.floor(minVal);
  let range = maxVal - minVal;

  // When data is pinned to a single integer (e.g. 93.00 → 93/93),
  // expand symmetrically so tiny sub-unit fluctuations stay visually flat.
  const MIN_VISIBLE_RANGE = 1;
  if (range < MIN_VISIBLE_RANGE) {
    maxVal = minVal + MIN_VISIBLE_RANGE;
    range = MIN_VISIBLE_RANGE;
  }

  // Time-based X axis: anchor to the last data point's timestamp
  // recorded_at is UTC from SQLite datetime('now'), append 'Z' for correct JS parsing
  const timestamps = points.map((p) => new Date(p.recorded_at + "Z").getTime());
  // Anchor tMax to the last data point rather than Date.now() to avoid
  // any drift between SQLite UTC timestamps and the JS clock, which can
  // cause the line to fall short of the right edge on first render.
  const tMax = timestamps[timestamps.length - 1];
  const tMin = tMax - rangeToMs(props.range);
  const tRange = tMax - tMin || 1;

  const toX = (i: number) => {
    const x = ((timestamps[i] - tMin) / tRange) * w;
    return Math.max(0, Math.min(w, x));
  };
  const toY = (v: number) =>
    padY + chartH - ((v - minVal) / range) * chartH;

  // Y-axis labels
  ctx.fillStyle = labelColor;
  ctx.font = "9px -apple-system, sans-serif";
  ctx.textAlign = "left";
  ctx.fillText(formatVal(maxVal, props.currency), 2, toY(maxVal) - 3);
  ctx.fillText(formatVal(minVal, props.currency), 2, toY(minVal) - 3);

  if (points.length === 1) {
    const x = toX(0);
    const y = toY(values[0]);
    ctx.fillStyle = "#4a9";
    ctx.beginPath();
    ctx.arc(x, y, 2.5, 0, Math.PI * 2);
    ctx.fill();
    return;
  }

  let linePath = "";
  let areaPath = "";
  for (let i = 0; i < points.length; i++) {
    const x = toX(i);
    const y = toY(values[i]);
    if (i === 0) {
      linePath += `M${x},${y}`;
      areaPath += `M${x},${y}`;
    } else {
      linePath += ` L${x},${y}`;
      areaPath += ` L${x},${y}`;
    }
  }
  const lastX = toX(points.length - 1);
  areaPath += ` L${lastX},${h} L${toX(0)},${h} Z`;

  const gradient = ctx.createLinearGradient(0, padY, 0, h);
  gradient.addColorStop(0, "rgba(68, 170, 153, 0.22)");
  gradient.addColorStop(0.6, "rgba(68, 170, 153, 0.06)");
  gradient.addColorStop(1, "rgba(68, 170, 153, 0.01)");

  ctx.fillStyle = gradient;
  ctx.fill(new Path2D(areaPath));

  ctx.strokeStyle = "#4a9";
  ctx.lineWidth = 1.8;
  ctx.lineJoin = "round";
  ctx.lineCap = "round";
  ctx.stroke(new Path2D(linePath));
}


onMounted(() => {
  nextTick(draw);
});

watch(
  [() => props.dataPoints, () => props.range],
  () => {
    nextTick(draw);
  },
  { deep: true }
);
</script>

<style scoped>
.trend-chart {
  width: 100%;
  height: 52px;
}
.trend-chart canvas {
  width: 100%;
  height: 52px;
}
</style>

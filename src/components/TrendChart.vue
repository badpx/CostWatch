<template>
  <div class="trend-chart" ref="containerRef">
    <canvas ref="canvasRef" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, nextTick } from "vue";
import type { HistoryPoint } from "../types";

const props = defineProps<{
  dataPoints: HistoryPoint[];
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

  const points = props.dataPoints;
  if (points.length === 0) return;

  const values = points.map((p) => p.value);
  const minVal = Math.min(...values);
  const maxVal = Math.max(...values);
  const range = maxVal - minVal || 1;

  const padY = 10;
  const chartH = h - padY * 2;

  // Time-based X axis: map timestamps to proportional positions
  const timestamps = points.map((p) => new Date(p.recorded_at).getTime());
  const tMin = Math.min(...timestamps);
  const tMax = Math.max(...timestamps);
  const tRange = tMax - tMin || 1;

  const toX = (i: number) =>
    points.length === 1 ? w / 2 : ((timestamps[i] - tMin) / tRange) * w;
  const toY = (v: number) =>
    padY + chartH - ((v - minVal) / range) * chartH;

  ctx.clearRect(0, 0, w, h);

  // Grid lines
  ctx.strokeStyle = "rgba(255,255,255,0.06)";
  ctx.lineWidth = 0.5;
  ctx.beginPath();
  ctx.moveTo(0, toY(minVal));
  ctx.lineTo(w, toY(minVal));
  ctx.moveTo(0, toY(maxVal));
  ctx.lineTo(w, toY(maxVal));
  ctx.stroke();

  // Y-axis labels
  ctx.fillStyle = "#666";
  ctx.font = "9px -apple-system, sans-serif";
  ctx.textAlign = "left";
  ctx.fillText(formatVal(maxVal), 2, toY(maxVal) - 3);
  ctx.fillText(formatVal(minVal), 2, toY(minVal) - 3);

  if (points.length === 1) {
    const y = toY(values[0]);
    ctx.strokeStyle = "#4a9";
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(toX(0) - 20, y);
    ctx.lineTo(toX(0) + 20, y);
    ctx.stroke();
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
  areaPath += ` L${toX(points.length - 1)},${h} L0,${h} Z`;

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

function formatVal(v: number): string {
  if (Math.abs(v) >= 1000) return `$${(v / 1000).toFixed(1)}k`;
  return `$${v.toFixed(v < 10 ? 2 : 0)}`;
}

onMounted(() => {
  nextTick(draw);
});

watch(
  () => props.dataPoints,
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

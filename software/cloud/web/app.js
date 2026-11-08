import { add, subtract, dot, norm, parseState, transform, legJoints } from "./kinematics.js";
import { PoseEditor } from "./editor.js";

const canvas = document.getElementById("scene");
const context = canvas.getContext("2d");
const empty = document.getElementById("empty");
const connection = document.getElementById("connection");
const workspace = document.getElementById("workspace");
const editorPanel = document.getElementById("editor-panel");
const copyState = document.getElementById("copy-state");
const liveTab = document.getElementById("live-tab");
const editorTab = document.getElementById("editor-tab");

let liveState = null;
let editing = false;
let pendingFrame = false;
let dragging = null;
let geometry = null;
let geometryDirty = true;
let handles = [];
let size = { width: 0, height: 0 };
const initialCamera = () => ({ azimuth: -0.95, elevation: 0.55, distance: 1.15, focus: [0, 0, 0] });
const cameras = { live: initialCamera(), editor: initialCamera() };
let camera = cameras.live;
const editor = new PoseEditor(() => { geometryDirty = true; requestDraw(); });
const activeState = () => editing ? editor.pose : liveState;
const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const unit = (v) => v.map((value) => value / norm(v));

function switchTab(toEditor) {
  editing = toEditor;
  camera = editing ? cameras.editor : cameras.live;
  if (editing && !editor.pose && liveState) {
    editor.load(liveState);
    camera.focus = [...liveState.body.position];
  }
  workspace.classList.toggle("editing", editing);
  editorPanel.hidden = !editing;
  liveTab.setAttribute("aria-pressed", String(!editing));
  editorTab.setAttribute("aria-pressed", String(editing));
  document.getElementById("view-help").textContent = editing
    ? "Trage componenta selectată · Dreapta / Shift: pan · Rotiță: zoom"
    : "Stânga: rotire · Dreapta / Shift: pan · Rotiță: zoom";
  geometryDirty = true;
  requestDraw();
}
liveTab.addEventListener("click", () => switchTab(false));
editorTab.addEventListener("click", () => switchTab(true));
copyState.addEventListener("click", () => { if (liveState) editor.load(liveState); });
document.getElementById("reset-view").addEventListener("click", () => {
  Object.assign(camera, initialCamera(), { focus: [...(activeState()?.body.position ?? [0, 0, 0])] });
  requestDraw();
});

function setConnection(status, text) {
  connection.dataset.status = status;
  connection.textContent = text;
}

const events = new EventSource("/events");
events.onopen = () => setConnection("online", "Conectat · aștept STATE");
events.onerror = () => setConnection("offline", "Conexiune întreruptă · reconectare");
events.onmessage = (message) => {
  try {
    const next = parseState(JSON.parse(message.data));
    if (!next) throw new Error("STATE invalid");
    if (!liveState) cameras.live.focus = [...next.body.position];
    liveState = next;
    copyState.disabled = false;
    setConnection("online", "STATE în timp real");
    if (editing && !editor.pose) {
      editor.load(next);
      cameras.editor.focus = [...next.body.position];
    }
    if (!editing) { geometryDirty = true; requestDraw(); }
  } catch (error) {
    setConnection("offline", "STATE invalid");
    console.error(error);
  }
};

function resize() {
  const rect = canvas.getBoundingClientRect();
  size = { width: rect.width, height: rect.height };
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const width = Math.round(rect.width * dpr);
  const height = Math.round(rect.height * dpr);
  if (canvas.width !== width) canvas.width = width;
  if (canvas.height !== height) canvas.height = height;
  context.setTransform(dpr, 0, 0, dpr, 0, 0);
  draw();
}
new ResizeObserver(resize).observe(canvas);

function requestDraw() {
  if (pendingFrame) return;
  pendingFrame = true;
  requestAnimationFrame(() => {
    pendingFrame = false;
    draw();
  });
}

function viewpoint() {
  const { azimuth: a, elevation: e, distance: d, focus } = camera;
  const eye = add(focus, [d * Math.cos(a) * Math.cos(e), d * Math.sin(a) * Math.cos(e), d * Math.sin(e)]);
  const forward = unit(subtract(focus, eye));
  const right = unit(cross(forward, [0, 0, 1]));
  const up = cross(right, forward);
  return { eye, forward, right, up };
}

function projector(width, height) {
  const view = viewpoint();
  const focal = Math.min(width, height) * 1.25;
  return (world) => {
    const relative = subtract(world, view.eye);
    const depth = dot(relative, view.forward);
    if (depth < 0.1) return null;
    return {
      x: width * 0.5 + (dot(relative, view.right) * focal) / depth,
      y: height * 0.52 - (dot(relative, view.up) * focal) / depth,
      depth,
      scale: focal / depth,
    };
  };
}

function line(a, b, color, width = 1, dashed = false) {
  if (!a || !b) return;
  context.beginPath();
  context.setLineDash(dashed ? [5, 5] : []);
  context.lineCap = "round";
  context.lineWidth = width;
  context.strokeStyle = color;
  context.moveTo(a.x, a.y);
  context.lineTo(b.x, b.y);
  context.stroke();
  context.setLineDash([]);
}

function circle(point, size, color, outline = null) {
  if (!point) return;
  context.beginPath();
  context.arc(point.x, point.y, Math.max(2, Math.min(13, size * point.scale)), 0, Math.PI * 2);
  context.fillStyle = color;
  context.fill();
  if (outline) {
    context.strokeStyle = outline;
    context.lineWidth = 1.5;
    context.stroke();
  }
}

function drawGrid(project) {
  const spacing = camera.distance < 1.6 ? 0.1 : camera.distance < 6 ? 0.25 : 1;
  const half = Math.max(7, Math.min(30, Math.ceil(camera.distance * 1.4 / spacing)));
  const [cx, cy] = camera.focus;
  const x0 = Math.floor(cx / spacing) - half;
  const x1 = Math.floor(cx / spacing) + half;
  const y0 = Math.floor(cy / spacing) - half;
  const y1 = Math.floor(cy / spacing) + half;
  for (let x = x0; x <= x1; x++) line(project([x * spacing, y0 * spacing, 0]), project([x * spacing, y1 * spacing, 0]), x === 0 ? "#568c8f59" : "#47637b29");
  for (let y = y0; y <= y1; y++) line(project([x0 * spacing, y * spacing, 0]), project([x1 * spacing, y * spacing, 0]), y === 0 ? "#568c8f59" : "#47637b29");
  const origin = project([0, 0, 0]);
  for (const [axis, point, color] of [["X", [spacing, 0, 0], "#eb9375"], ["Y", [0, spacing, 0], "#76d9ac"], ["Z", [0, 0, spacing], "#7aa6ee"]]) {
    const end = project(point);
    line(origin, end, color, 2);
    if (end) {
      context.fillStyle = color;
      context.font = "bold 11px system-ui";
      context.fillText(axis, end.x + 6, end.y - 5);
    }
  }
}

function prepareGeometry(state) {
  const bodyToWorld = transform(state.body.position, state.body.rotation).toWorld;
  // Conturul carcasei trece prin motoarele primite în STATE, în ordine în jurul centrului.
  const mounts = state.legs.map((leg) => leg.motor).filter((point, i, all) =>
    all.findIndex((other) => norm(subtract(point, other)) < 1e-9) === i);
  const centroid = mounts.reduce((sum, point) => add(sum, point), [0, 0, 0]).map((value) => value / mounts.length);
  mounts.sort((a, b) => Math.atan2(a[1] - centroid[1], a[0] - centroid[0])
    - Math.atan2(b[1] - centroid[1], b[0] - centroid[0]));
  const area = mounts.reduce((sum, point, i) => {
    const next = mounts[(i + 1) % mounts.length];
    return sum + point[0] * next[1] - next[0] * point[1];
  }, 0);
  const outline = mounts.length >= 3 && Math.abs(area) > 1e-10 ? mounts : [];
  const radius = Math.max(...mounts.map((point) => Math.hypot(point[0], point[1])));
  const thickness = Math.max(0.004, radius * 0.12);
  return {
    top: outline.map(bodyToWorld),
    bottom: outline.map(([x, y, z]) => bodyToWorld([x, y, z - thickness])),
    center: bodyToWorld([0, 0, thickness * 0.6]),
    front: bodyToWorld([radius * 0.55, 0, thickness * 0.6]),
    legs: state.legs.map((leg) => {
      const joints = legJoints(leg).map(bodyToWorld);
      return { joints, foot: bodyToWorld(leg.foot) };
    }),
  };
}

function drawBody(project, primitives) {
  const { top, bottom } = geometry;
  for (let i = 0; i < top.length; i++) {
    const corners = [bottom[i], bottom[(i + 1) % top.length], top[(i + 1) % top.length], top[i]].map(project);
    if (corners.every(Boolean)) {
      primitives.push({ depth: corners.reduce((sum, c) => sum + c.depth, 0) / 4, draw() {
        context.beginPath();
        context.moveTo(corners[0].x, corners[0].y);
        for (const corner of corners.slice(1)) context.lineTo(corner.x, corner.y);
        context.closePath();
        context.fillStyle = i % 2 ? "#234453" : "#2b5661";
        context.fill();
        context.strokeStyle = "#6ca6a664";
        context.lineWidth = 1;
        context.stroke();
      } });
    }
  }
  const face = top.map(project);
  if (face.length >= 3 && face.every(Boolean)) {
    primitives.push({ depth: face.reduce((sum, c) => sum + c.depth, 0) / face.length, draw() {
      context.beginPath();
      context.moveTo(face[0].x, face[0].y);
      for (const corner of face.slice(1)) context.lineTo(corner.x, corner.y);
      context.closePath();
      context.fillStyle = "#305a68";
      context.fill();
      context.lineWidth = 2;
      context.strokeStyle = "#83d1c5";
      context.stroke();
    } });
  }
  const center = project(geometry.center);
  const front = project(geometry.front);
  if (center && front) primitives.push({ depth: center.depth - 0.01, draw() { line(center, front, "#f1b674", 3); circle(center, 0.07, "#c6efdc"); } });
}

function drawLegs(project, primitives) {
  geometry.legs.forEach(({ joints: worldJoints, foot }, index) => {
    const joints = worldJoints.map(project);
    const reported = project(foot);
    const selected = editing && editor.selection.endsWith(`:${index}`);
    const color = selected ? "#f6d276" : index % 2 === 0 ? "#81d7d0" : "#9fbdda";

    for (let segment = 0; segment < 3; segment++) {
      const a = joints[segment];
      const b = joints[segment + 1];
      if (!a || !b) continue;
      primitives.push({ depth: (a.depth + b.depth) / 2, draw() {
        const thickness = Math.min(12, Math.max(4, 0.07 * (a.scale + b.scale)));
        line(a, b, "#0a1a28", thickness + 3);
        line(a, b, segment === 2 ? "#f3b17a" : color, thickness);
      } });
    }
    joints.forEach((joint, jointIndex) => {
      if (joint) primitives.push({ depth: joint.depth - 0.01, draw() {
        circle(joint, jointIndex === 3 ? 0.085 : 0.07, jointIndex === 3 ? "#ffc189" : "#b4e8de", "#0b2532");
      } });
    });
    // Un vârf raportat incompatibil cu unghiurile rămâne vizibil ca reper,
    // fără să întindem artificial ultimul segment până la el.
    if (reported && norm(subtract(foot, worldJoints[3])) > 0.005) {
      primitives.push({ depth: reported.depth - 0.01, draw() {
        circle(reported, 0.06, "#142938", "#cde5e5");
      } });
    }
    if (joints[3]) {
      const label = joints[3];
      primitives.push({ depth: label.depth - 0.02, draw() {
        context.font = "bold 12px system-ui";
        context.fillStyle = "#f4ead9";
        context.fillText(String(index + 1), label.x + 8 + (index % 3) * 7, label.y - 8 - Math.floor(index / 3) * 13);
      } });
    }
  });
}

function draw() {
  const { width, height } = size;
  context.clearRect(0, 0, width, height);
  if (!width || !height) return;

  handles = [];
  const state = activeState();
  empty.hidden = Boolean(state);
  const project = projector(width, height);
  drawGrid(project);
  if (!state) return;
  if (geometryDirty) {
    geometry = prepareGeometry(state);
    geometryDirty = false;
  }

  const primitives = [];
  drawBody(project, primitives);
  drawLegs(project, primitives);
  primitives.sort((a, b) => b.depth - a.depth);
  for (const primitive of primitives) primitive.draw();
  if (editing) drawHandles(project);
}

function drawHandles(project) {
  const points = [{ key: "body", world: editor.pose.body.position }];
  geometry.legs.forEach(({ joints, foot }, i) => {
    points.push({ key: `mount:${i}`, world: joints[0] }, { key: `foot:${i}`, world: foot });
  });
  handles = points.map((handle) => ({ ...handle, screen: project(handle.world) })).filter((handle) => handle.screen);
  const selected = handles.find((handle) => handle.key === editor.selection);
  if (!selected) return;
  const { x, y } = selected.screen;
  context.beginPath();
  context.arc(x, y, 12, 0, 2 * Math.PI);
  context.strokeStyle = "#ffe5a1";
  context.lineWidth = 2;
  context.stroke();
  context.fillStyle = "#ffe5a1";
  context.font = "11px system-ui";
  context.fillText(`↔ ${editor.plane.value.toUpperCase()}`, x + 17, y + 18);
}

function pick(point) {
  return handles
    .map((handle) => ({ ...handle, distance: Math.hypot(handle.screen.x - point[0], handle.screen.y - point[1]) }))
    .filter((handle) => handle.distance <= 16)
    .sort((a, b) => Number(b.key === editor.selection) - Number(a.key === editor.selection)
      || a.distance - b.distance || a.screen.depth - b.screen.depth)[0];
}

function pointerPosition(event) {
  const rect = canvas.getBoundingClientRect();
  return [event.clientX - rect.left, event.clientY - rect.top];
}

function intersectPlane(point, anchor, normal) {
  const { eye, forward, right, up } = viewpoint();
  const focal = Math.min(size.width, size.height) * 1.25;
  const dx = (point[0] - size.width * 0.5) / focal;
  const dy = (size.height * 0.52 - point[1]) / focal;
  const ray = forward.map((v, i) => v + dx * right[i] + dy * up[i]);
  const denominator = dot(ray, normal);
  if (Math.abs(denominator) < 1e-6) return null;
  const t = dot(subtract(anchor, eye), normal) / denominator;
  return t > 0 ? eye.map((v, i) => v + t * ray[i]) : null;
}

canvas.addEventListener("contextmenu", (event) => event.preventDefault());
canvas.addEventListener("pointerdown", (event) => {
  if (dragging || event.button > 2) return;
  event.preventDefault();
  const pan = event.button !== 0 || event.shiftKey;
  dragging = { x: event.clientX, y: event.clientY, id: event.pointerId, mode: pan ? "pan" : "orbit" };
  if (editing && !pan) {
    const point = pointerPosition(event);
    const handle = pick(point);
    if (handle) {
      editor.select(handle.key);
      const anchor = editor.point();
      const normal = { xy: [0, 0, 1], xz: [0, 1, 0], yz: [1, 0, 0] }[editor.plane.value];
      const start = intersectPlane(point, anchor, normal);
      if (start) Object.assign(dragging, { mode: "component", anchor, normal, offset: subtract(anchor, start) });
      else {
        dragging.mode = "idle";
        editor.notify("Planul este văzut din lateral. Rotește vederea sau alege alt plan.", true);
      }
    }
  }
  canvas.setPointerCapture(event.pointerId);
});
canvas.addEventListener("pointermove", (event) => {
  if (!dragging || dragging.id !== event.pointerId) return;
  const dx = event.clientX - dragging.x;
  const dy = event.clientY - dragging.y;
  if (dragging.mode === "pan") {
    const { right, up } = viewpoint();
    const scale = camera.distance / (Math.min(size.width, size.height) * 1.25);
    camera.focus = camera.focus.map((v, i) => v - scale * dx * right[i] + scale * dy * up[i]);
  } else if (dragging.mode === "orbit") {
    camera.azimuth += dx * 0.006;
    camera.elevation = Math.max(-1.3, Math.min(1.3, camera.elevation + dy * 0.006));
  } else if (dragging.mode === "component") {
    const point = intersectPlane(pointerPosition(event), dragging.anchor, dragging.normal);
    if (point) editor.move(add(point, dragging.offset));
  }
  dragging.x = event.clientX;
  dragging.y = event.clientY;
  requestDraw();
});
for (const type of ["pointerup", "pointercancel", "lostpointercapture"]) {
  canvas.addEventListener(type, (event) => {
    if (dragging?.id === event.pointerId) dragging = null;
  });
}
canvas.addEventListener("wheel", (event) => {
  event.preventDefault();
  camera.distance = Math.max(0.4, Math.min(80, camera.distance * Math.exp(event.deltaY * 0.001)));
  requestDraw();
}, { passive: false });

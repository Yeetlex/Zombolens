// ZomboLens UI: one overlay with three modes, values on/off and a minimized dot.
const T = window.__TAURI__;
const invoke = (c, a) => T.core.invoke(c, a);
const win = T.window.getCurrentWindow();

// stat: label, max, higher is worse, moodle name and thresholds (approx., wiki B41)
const STAT = {
  hunger: ["Hambre", 1, true, "Hambre", [0.15, 0.25, 0.45, 0.7]],
  thirst: ["Sed", 1, true, "Sed", [0.12, 0.25, 0.7, 0.84]],
  fatigue: ["Sueño", 1, true, "Cansancio", [0.6, 0.7, 0.8, 0.9]],
  endurance: ["Aguante", 1, false, "Agotamiento", [0.75, 0.5, 0.25, 0.1]],
  stress: ["Estrés", 1, true, "Estrés", [0.25, 0.5, 0.75, 0.9]],
  panic: ["Pánico", 100, true, "Pánico", [6, 30, 65, 80]],
  pain: ["Dolor", 100, true, "Dolor", [10, 20, 50, 75]],
  boredom: ["Aburrimiento", 100, true, "Aburrimiento", [25, 50, 75, 90]],
  unhappiness: ["Infelicidad", 100, true, "Infelicidad", [20, 45, 60, 80]],
  wetness: ["Mojado", 100, true, "Mojado", [15, 40, 70, 90]],
  temperature: ["Temperatura", null, true],
  discomfort: ["Incomodidad", 100, true],
  idleness: ["Inactividad", 1, true],
  anger: ["Ira", 1, true, "Ira", [0.1, 0.3, 0.5, 0.7]],
  morale: ["Moral", 1, false],
  sanity: ["Cordura", 1, false],
  fitness: ["Forma física", 1, false],
  intoxication: ["Borrachera", 100, true, "Borracho", [10, 30, 50, 70]],
  nicotine_withdrawal: ["Mono de nicotina", 1, true, "Mono", [0.1, 0.3, 0.5, 0.7]],
  sickness: ["Enfermedad", 1, true, "Enfermo", [0.25, 0.5, 0.75, 0.9]],
  food_sickness: ["Intox. alimentaria", 100, true, "Intoxicado", [25, 50, 75, 90]],
  poison: ["Veneno", 100, true, "Envenenado", [10, 30, 50, 70]],
};
const BASIC = ["hunger", "thirst", "fatigue", "endurance", "stress", "panic", "pain", "boredom", "unhappiness", "wetness", "temperature"];
const ALL = ["hunger", "thirst", "fatigue", "endurance", "stress", "panic", "pain", "boredom", "unhappiness", "wetness",
  "temperature", "discomfort", "idleness", "anger", "morale", "sanity", "fitness", "intoxication", "nicotine_withdrawal",
  "sickness", "food_sickness", "poison"];
const PARTS_ES = {
  Hand_L: "Mano izq.", Hand_R: "Mano der.", ForeArm_L: "Antebrazo izq.", ForeArm_R: "Antebrazo der.",
  UpperArm_L: "Brazo izq.", UpperArm_R: "Brazo der.", Torso_Upper: "Pecho", Torso_Lower: "Abdomen", Head: "Cabeza",
  Neck: "Cuello", Groin: "Ingle", UpperLeg_L: "Muslo izq.", UpperLeg_R: "Muslo der.", LowerLeg_L: "Pierna izq.",
  LowerLeg_R: "Pierna der.", Foot_L: "Pie izq.", Foot_R: "Pie der.",
};

let settings = await invoke("get_settings");
let state = null;
const esc = (s) => String(s ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

function heat(frac, badHigh) {
  const f = Math.max(0, Math.min(1, frac));
  const bad = badHigh ? f : 1 - f;
  const hue = 120 - 120 * bad; // green -> red
  return `hsl(${hue}, 70%, 58%)`;
}

function statRow(key) {
  const def = STAT[key];
  const v = state?.stats?.[key];
  if (v == null) return "";
  let frac, text;
  if (key === "temperature") {
    frac = (v - 34) / 7;
    text = v.toFixed(1) + " °C";
  } else {
    frac = v / def[1];
    text = def[1] === 1 ? Math.round(v * 100) + "%" : v.toFixed(0);
  }
  const color = key === "temperature" ? (v < 36 || v > 37.6 ? "var(--warn)" : "var(--good)") : heat(frac, def[2]);
  return `<div class="row"><span class="k">${def[0]}</span><span class="bar"><i style="width:${Math.max(0, Math.min(1, frac)) * 100}%;background:${color}"></i></span><span class="v num">${text}</span></div>`;
}

const card = (title, accent, body, sub = "") =>
  `<div class="card" style="--accent:var(${accent})"><h3>${title}${sub ? `<span class="sub">${sub}</span>` : ""}</h3>${body}</div>`;
const plain = (k, v) => `<div class="row plain"><span class="k">${k}</span><span class="v num">${v}</span></div>`;
const fmt = (v, d = 0) => (v == null || !isFinite(v) ? "–" : Number(v).toFixed(d));

function cardState(keys) {
  const h = state?.health;
  const health = h == null ? "" :
    `<div class="row"><span class="k">Salud <span class="sub">aprox.</span></span><span class="bar"><i style="width:${h}%;background:${heat(h / 100, false)}"></i></span><span class="v num">${h.toFixed(0)}</span></div>`;
  return card("Estado", "--c-stats", health + keys.map(statRow).join(""));
}

function cardPosition() {
  const p = state?.position;
  if (!p) return card("Posición", "--c-pos", `<div class="empty">Muévete para verla</div>`);
  const coords = `${Math.floor(p[0])}, ${Math.floor(p[1])}, ${p[2]}`;
  return card("Posición", "--c-pos", `<div class="pos num">${coords}</div><div class="note">Como lo escriben los admins: x, y, planta</div>`);
}

function cardFood() {
  const s = state;
  return card("Nutrición", "--c-food",
    plain("Calorías", fmt(s?.calories)) + plain("Proteínas", fmt(s?.proteins) + " g") + plain("Grasas", fmt(s?.lipids) + " g") +
    plain("Carbohidratos", fmt(s?.carbohydrates) + " g") + plain("Peso", fmt(s?.weight, 1) + " kg"));
}

function cardOther() {
  const s = state;
  const meds = s?.effects ?? {};
  const active = [["sleep", "Somníferos"], ["beta", "Betabloqueantes"], ["depress", "Antidepresivos"], ["painkiller", "Analgésicos"]]
    .filter(([k]) => (meds[k] ?? 0) > 0.001).map(([, n]) => n);
  return card("Otros", "--c-meds",
    plain("Carga máxima", fmt(s?.maxWeight)) +
    plain("Velocidad andar / correr", `${fmt(s?.walkSpeed, 2)} / ${fmt(s?.runSpeed, 2)}`) +
    plain("Resfriado", s?.hasCold ? `sí (${fmt(s?.coldStrength)})` : "no") +
    plain("Medicamentos", active.length ? active.join(", ") : "ninguno"));
}

function cardBody() {
  const parts = (state?.parts ?? []).filter((p) => p.health < 99.95 || p.bandaged || p.bleeding || p.scratched || p.bitten || p.cut || p.deepWounded || p.fractureTime > 0 || p.burnTime > 0 || p.glass || p.bullet || p.splint || p.stitched);
  if (!parts.length) return card("Cuerpo", "--c-body", `<div class="empty">Sin heridas</div>`);
  const body = parts.map((p) => {
    const tags = [];
    if (p.bleeding) tags.push(["sangra", ""]);
    if (p.bitten) tags.push(["mordisco", ""]);
    if (p.scratched) tags.push(["arañazo", ""]);
    if (p.cut) tags.push(["corte", ""]);
    if (p.deepWounded) tags.push(["herida profunda", ""]);
    if (p.fractureTime > 0) tags.push(["fractura", ""]);
    if (p.burnTime > 0) tags.push(["quemadura", ""]);
    if (p.glass) tags.push(["cristal", ""]);
    if (p.bullet) tags.push(["bala", ""]);
    if (p.woundInfected) tags.push(["herida infectada", ""]);
    if (p.bandaged) tags.push([`venda ${Math.round((p.bandageLife ?? 0) * 100)}%`, (p.bandageLife ?? 0) > 0.3 ? "ok" : ""]);
    if (p.stitched) tags.push(["puntos", "ok"]);
    if (p.splint) tags.push(["férula", "ok"]);
    return `<div class="part"><span class="pn">${PARTS_ES[p.name] ?? p.name} <span class="num">${p.health.toFixed(0)}</span></span><span class="tags">${tags.map(([t, c]) => `<span class="tag ${c}">${t}</span>`).join("")}</span></div>`;
  }).join("");
  return card("Cuerpo", "--c-body", body);
}

function cardMoodles() {
  const chips = [];
  for (const [k, def] of Object.entries(STAT)) {
    const th = def[4], v = state?.stats?.[k];
    if (!th || v == null) continue;
    let level = 0;
    th.forEach((t, i) => { if (def[2] ? v >= t : v <= t) level = i + 1; });
    if (level) chips.push(`<span class="chip l${level}">${def[3]} ${"I".repeat(level)}</span>`);
  }
  return card("Moodles", "--c-moodle", chips.length ? `<div class="chips">${chips.join("")}</div>` : `<div class="empty">Ninguno</div>`, "estimados");
}

function render() {
  document.querySelectorAll(".tab[data-mode]").forEach((b) => b.classList.toggle("on", b.dataset.mode === settings.mode));
  document.getElementById("valuesBtn").classList.toggle("on", settings.showValues);
  document.body.classList.toggle("min", settings.minimized);
  const cols = document.getElementById("cols");
  cols.classList.toggle("hidden-values", !settings.showValues);
  let html;
  if (settings.mode === "basic") {
    html = `<div class="col">${cardState(BASIC)}${cardPosition()}</div>`;
  } else if (settings.mode === "extended") {
    html = `<div class="col">${cardState(ALL)}</div><div class="col">${cardFood()}${cardOther()}${cardPosition()}</div>`;
  } else {
    html = `<div class="col">${cardState(ALL)}</div><div class="col">${cardBody()}${cardMoodles()}${cardFood()}</div><div class="col">${cardOther()}${cardPosition()}</div>`;
  }
  cols.innerHTML = html;

  const dot = document.getElementById("dotStatus");
  const status = document.getElementById("status");
  if (state?.captureError) {
    dot.className = "dot"; status.textContent = "Sin captura: " + state.captureError;
  } else if (state?.ageS == null) {
    dot.className = "dot"; status.textContent = state?.server ? "Conectado, esperando datos…" : "Esperando al juego…";
  } else if (state.ageS > 5) {
    dot.className = "dot stale"; status.textContent = `Sin datos hace ${Math.round(state.ageS)} s`;
  } else {
    dot.className = "dot ok"; status.textContent = (state.replay ? "Reproducción · " : "") + state.server;
  }
  document.getElementById("dot").classList.toggle("alert", (state?.parts ?? []).some((p) => p.bleeding));
  fitWindow();
}

// The window follows its content.
let lastSize = "";
function fitWindow() {
  const el = settings.minimized ? document.getElementById("dot") : document.getElementById("app");
  const r = el.getBoundingClientRect();
  const w = Math.ceil(r.width) + 2, h = Math.ceil(r.height) + 2;
  const key = `${w}x${h}`;
  if (key !== lastSize && w > 10 && h > 10) {
    lastSize = key;
    invoke("fit_window", { width: w, height: h });
  }
}

function save() { invoke("save_settings", { settings }); render(); }
document.querySelectorAll(".tab[data-mode]").forEach((b) => (b.onclick = () => { settings.mode = b.dataset.mode; save(); }));
document.getElementById("valuesBtn").onclick = () => { settings.showValues = !settings.showValues; save(); };
document.getElementById("minBtn").onclick = () => { settings.minimized = true; save(); };

// The dot: a click opens, a drag moves.
const dotEl = document.getElementById("dot");
let down = null;
dotEl.addEventListener("mousedown", (e) => { down = { x: e.screenX, y: e.screenY }; });
dotEl.addEventListener("mousemove", (e) => {
  if (down && (Math.abs(e.screenX - down.x) > 4 || Math.abs(e.screenY - down.y) > 4)) { down = null; win.startDragging(); }
});
dotEl.addEventListener("mouseup", () => { if (down) { settings.minimized = false; save(); } down = null; });

T.event.listen("state", (e) => { state = e.payload; render(); });
render();

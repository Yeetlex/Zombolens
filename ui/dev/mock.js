// Stand-in for Tauri in a plain browser: a hurt, tipsy, bandaged character.
(() => {
  const mode = new URLSearchParams(location.search).get("mode") || "total";
  const settings = { mode, showValues: true, minimized: location.search.includes("min"), x: null, y: null };
  const stats = { hunger: 0.31, thirst: 0.18, fatigue: 0.05, endurance: 0.42, stress: 0.35, panic: 12, pain: 27, boredom: 46,
    unhappiness: 22, wetness: 55, temperature: 36.4, discomfort: 18, idleness: 0.2, anger: 0.12, morale: 0.8, sanity: 0.95,
    fitness: 0.3, intoxication: 33, nicotine_withdrawal: 0, sickness: 0.05, food_sickness: 0, poison: 0 };
  const part = (name, health, extra = {}) => ({ name, health, cut: false, bitten: false, scratched: false, bandaged: false, bleeding: false,
    deepWounded: false, bandageLife: null, woundInfected: false, glass: false,
    stitched: false, fractureTime: 0, splint: false, bullet: false, burnTime: 0, needBurnWash: false, bandageType: "", splintItem: "", ...extra });
  const state = { server: "127.0.0.1:16262", ageS: 0.4, replay: false, captureError: null, stats,
    calories: 740, proteins: -3.2, lipids: -4.2, carbohydrates: -13, weight: 80.2, health: 92.4, partHealth: [],
    parts: [part("ForeArm_R", 88.6, { scratched: true, bleeding: true, bandaged: true, bandageLife: 0.55, bandageType: "Base.RippedSheets" }),
      part("LowerLeg_L", 71, { bitten: true, woundInfected: true }), part("Hand_L", 100, { stitched: true, cut: true })],
    maxWeight: 12, walkSpeed: 0.5, runSpeed: 0.35, hasCold: true, coldStrength: 34, effects: { sleep: 0, beta: 0, depress: 0, painkiller: 0.6 },
    position: [10721.4, 10630.7, 1] };
  const handlers = {};
  window.__TAURI__ = {
    core: { invoke: async (c, a) => (c === "get_settings" ? settings : null) },
    event: { listen: async (ev, cb) => { (handlers[ev] ||= []).push(cb); } },
    window: { getCurrentWindow: () => ({ setSize() {}, startDragging() {} }) },
    dpi: { LogicalSize: class { constructor(w, h) { this.w = w; this.h = h; } } },
  };
  setInterval(() => { state.stats.hunger = Math.min(1, state.stats.hunger + 0.002); (handlers.state || []).forEach((cb) => cb({ payload: state })); }, 500);
})();
